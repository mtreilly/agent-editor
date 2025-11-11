package cmd

import (
	"archive/tar"
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"strings"
	"time"

	"github.com/agent-editor/agent-editor/cli/internal/config"
	"github.com/agent-editor/agent-editor/cli/internal/output"
	"github.com/agent-editor/agent-editor/cli/internal/rpc"
	"github.com/spf13/cobra"
)

func exportCmd() *cobra.Command {
	export := &cobra.Command{Use: "export", Short: "Export data"}

	docs := &cobra.Command{Use: "docs", RunE: func(cmd *cobra.Command, args []string) error {
		repo, _ := cmd.Flags().GetString("repo")
		includeDeleted, _ := cmd.Flags().GetBool("include-deleted")
		includeVersionsFlag, _ := cmd.Flags().GetBool("include-versions")
		outFile, _ := cmd.Flags().GetString("out")
		format, _ := cmd.Flags().GetString("format")
		format = strings.ToLower(format)
		if format == "" {
			format = "json"
		}
		switch format {
		case "json", "jsonl", "tar":
		default:
			return fmt.Errorf("invalid --format %s (expected json|jsonl|tar)", format)
		}
		if (format == "jsonl" || format == "tar") && outFile == "" {
			return fmt.Errorf("--out is required when format=%s", format)
		}
		if format == "tar" {
			includeVersionsFlag = true
		}

		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		params := map[string]interface{}{}
		if repo != "" {
			params["repo_id"] = repo
		}
		if includeDeleted {
			params["include_deleted"] = true
		}
		if includeVersionsFlag {
			params["include_versions"] = true
		}
		var res []map[string]interface{}
		if err := cli.Call(ctx, "export_docs", params, &res); err != nil {
			return err
		}

		if outFile != "" {
			switch format {
			case "json":
				data, err := json.MarshalIndent(res, "", "  ")
				if err != nil {
					return err
				}
				if err := os.WriteFile(outFile, data, 0o644); err != nil {
					return err
				}
			case "jsonl":
				f, err := os.Create(outFile)
				if err != nil {
					return err
				}
				defer f.Close()
				w := bufio.NewWriter(f)
				for _, row := range res {
					line, err := json.Marshal(row)
					if err != nil {
						return err
					}
					if _, err := w.Write(append(line, '\n')); err != nil {
						return err
					}
				}
				if err := w.Flush(); err != nil {
					return err
				}
			case "tar":
				if err := writeDocsTar(outFile, res); err != nil {
					return err
				}
			}
			return output.Print(fmt.Sprintf("exported %d docs to %s (%s)", len(res), outFile, format), cfg.OutputFormat)
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	docs.Flags().String("repo", "", "Repo ID to filter (default: all repos)")
	docs.Flags().Bool("include-deleted", false, "Include docs marked as deleted")
	docs.Flags().String("out", "", "Write export to file (required for jsonl/tar)")
	docs.Flags().String("format", "json", "Output format when using --out (json|jsonl|tar)")
	docs.Flags().Bool("include-versions", false, "Include version metadata (auto-enabled for tar)")

	db := &cobra.Command{Use: "db", RunE: func(cmd *cobra.Command, args []string) error {
		outFile, _ := cmd.Flags().GetString("out")
		if outFile == "" {
			return fmt.Errorf("--out is required")
		}
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res map[string]interface{}
		if err := cli.Call(ctx, "export_db", map[string]interface{}{"out_path": outFile}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	db.Flags().String("out", "", "Destination path for SQLite backup")

	export.AddCommand(docs, db)
	return export
}

var sanitizedFilename = regexp.MustCompile(`[^a-zA-Z0-9-_]`)

func safeDocFilename(id, slug string) string {
	slug = strings.ToLower(slug)
	slug = sanitizedFilename.ReplaceAllString(slug, "-")
	if slug == "" {
		slug = "doc"
	}
	if len(slug) > 40 {
		slug = slug[:40]
	}
	return fmt.Sprintf("docs/%s-%s.md", slug, id)
}

func writeDocsTar(out string, docs []map[string]interface{}) error {
	f, err := os.Create(out)
	if err != nil {
		return err
	}
	defer f.Close()
	tw := tar.NewWriter(f)
	defer tw.Close()

	docsJSON, err := json.MarshalIndent(docs, "", "  ")
	if err != nil {
		return err
	}
	if err := tw.WriteHeader(&tar.Header{Name: "docs.json", Mode: 0o600, Size: int64(len(docsJSON))}); err != nil {
		return err
	}
	if _, err := tw.Write(docsJSON); err != nil {
		return err
	}

	meta := map[string]interface{}{
		"created_at": time.Now().UTC().Format(time.RFC3339),
		"doc_count":  len(docs),
		"format":     "json",
		"version":    "1",
	}
	metaJSON, err := json.MarshalIndent(meta, "", "  ")
	if err != nil {
		return err
	}
	if err := tw.WriteHeader(&tar.Header{Name: "meta.json", Mode: 0o600, Size: int64(len(metaJSON))}); err != nil {
		return err
	}
	if _, err := tw.Write(metaJSON); err != nil {
		return err
	}

	var versionsPayload []map[string]interface{}
	for _, doc := range docs {
		if vers, ok := doc["versions"].([]interface{}); ok && len(vers) > 0 {
			versionsPayload = append(versionsPayload, map[string]interface{}{
				"doc_id":   doc["id"],
				"versions": vers,
			})
		}
	}
	if len(versionsPayload) > 0 {
		versionsJSON, err := json.MarshalIndent(versionsPayload, "", "  ")
		if err != nil {
			return err
		}
		if err := tw.WriteHeader(&tar.Header{Name: "versions.json", Mode: 0o600, Size: int64(len(versionsJSON))}); err != nil {
			return err
		}
		if _, err := tw.Write(versionsJSON); err != nil {
			return err
		}
	}
	for _, doc := range docs {
		body, _ := doc["body"].(string)
		if body == "" {
			continue
		}
		id, _ := doc["id"].(string)
		slug, _ := doc["slug"].(string)
		filename := safeDocFilename(id, slug)
		content := []byte(body)
		if err := tw.WriteHeader(&tar.Header{Name: filename, Mode: 0o600, Size: int64(len(content))}); err != nil {
			return err
		}
		if _, err := tw.Write(content); err != nil {
			return err
		}
	}
	return nil
}
