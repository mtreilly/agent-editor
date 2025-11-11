package cmd

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"github.com/agent-editor/agent-editor/cli/internal/config"
	"github.com/agent-editor/agent-editor/cli/internal/output"
	"github.com/agent-editor/agent-editor/cli/internal/rpc"
	"github.com/spf13/cobra"
)

func importCmd() *cobra.Command {
	importRoot := &cobra.Command{Use: "import", Short: "Import archives"}

	docs := &cobra.Command{
		Use:   "docs <path>",
		Short: "Import docs from archive",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			repo, _ := cmd.Flags().GetString("repo")
			newRepo, _ := cmd.Flags().GetString("new-repo")
			dryRun, _ := cmd.Flags().GetBool("dry-run")
			mergeStrategy, _ := cmd.Flags().GetString("merge-strategy")
			mergeStrategy = strings.ToLower(mergeStrategy)
			if repo == "" && newRepo == "" {
				return fmt.Errorf("specify --repo or --new-repo")
			}
			if repo != "" && newRepo != "" {
				return fmt.Errorf("--repo and --new-repo are mutually exclusive")
			}
			if mergeStrategy != "keep" && mergeStrategy != "overwrite" {
				return fmt.Errorf("invalid --merge-strategy %s", mergeStrategy)
			}
			cfg := config.Load()
			cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)

			progressPath, cleanup, err := createProgressLog()
			if err != nil {
				return err
			}
			defer cleanup()
			ctx, cancel := context.WithCancel(cmd.Context())
			defer cancel()
			if progressPath != "" {
				go streamImportProgress(ctx, progressPath)
			}

			payload := map[string]interface{}{
				"path":           args[0],
				"dry_run":        dryRun,
				"merge_strategy": mergeStrategy,
			}
			if repo != "" {
				payload["repo_id"] = repo
			}
			if newRepo != "" {
				payload["new_repo_name"] = newRepo
			}
			if progressPath != "" {
				payload["progress_path"] = progressPath
			}
			var res map[string]interface{}
			if err := cli.Call(ctx, "import_docs", payload, &res); err != nil {
				return err
			}
			cancel()
			// Allow progress goroutine to drain final events
			time.Sleep(150 * time.Millisecond)
			return output.Print(res, cfg.OutputFormat)
		},
	}
	docs.Flags().String("repo", "", "Existing repo to import into")
	docs.Flags().String("new-repo", "", "Create a new repo for import")
	docs.Flags().Bool("dry-run", true, "Validate without writing to DB (set --dry-run=false to apply)")
	docs.Flags().String("merge-strategy", "keep", "Conflict strategy: keep|overwrite")

	importRoot.AddCommand(docs)
	return importRoot
}

type importProgressEvent struct {
	Status    string `json:"status"`
	Processed int    `json:"processed"`
	Total     int    `json:"total"`
	Inserted  int    `json:"inserted"`
	Updated   int    `json:"updated"`
	Skipped   int    `json:"skipped"`
}

func createProgressLog() (string, func(), error) {
	f, err := os.CreateTemp("", "agent-editor-import-progress-*.log")
	if err != nil {
		return "", func() {}, err
	}
	path := f.Name()
	f.Close()
	cleanup := func() {
		_ = os.Remove(path)
	}
	return path, cleanup, nil
}

func streamImportProgress(ctx context.Context, path string) {
	ticker := time.NewTicker(250 * time.Millisecond)
	defer ticker.Stop()
	var offset int64
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			offset = readProgressChunk(path, offset)
		}
	}
}

func readProgressChunk(path string, offset int64) int64 {
	f, err := os.Open(path)
	if err != nil {
		return offset
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil {
		return offset
	}
	if info.Size() <= offset {
		return offset
	}
	if _, err := f.Seek(offset, io.SeekStart); err != nil {
		return offset
	}
	reader := bufio.NewReader(f)
	for {
		line, err := reader.ReadString('\n')
		if err != nil {
			if err == io.EOF {
				break
			}
			break
		}
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		var evt importProgressEvent
		if err := json.Unmarshal([]byte(line), &evt); err == nil && evt.Total > 0 {
			fmt.Fprintf(os.Stderr, "[import] %s %d/%d inserted=%d updated=%d skipped=%d\n",
				strings.ToUpper(evt.Status), evt.Processed, evt.Total, evt.Inserted, evt.Updated, evt.Skipped)
		} else {
			fmt.Fprintf(os.Stderr, "[import] %s\n", line)
		}
	}
	newOffset, err := f.Seek(0, io.SeekCurrent)
	if err != nil {
		return offset
	}
	return newOffset
}
