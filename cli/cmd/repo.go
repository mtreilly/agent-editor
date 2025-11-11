package cmd

import (
	"context"
	"fmt"
	"time"

	"github.com/agent-editor/agent-editor/cli/internal/config"
	"github.com/agent-editor/agent-editor/cli/internal/output"
	"github.com/agent-editor/agent-editor/cli/internal/rpc"
	"github.com/spf13/cobra"
)

func repoCmd() *cobra.Command {
	repo := &cobra.Command{Use: "repo", Short: "Manage repositories"}

	add := &cobra.Command{
		Use:   "add <path>",
		Args:  cobra.ExactArgs(1),
		Short: "Register a repo path",
		RunE: func(cmd *cobra.Command, args []string) error {
			name, _ := cmd.Flags().GetString("name")
			include, _ := cmd.Flags().GetStringArray("include")
			exclude, _ := cmd.Flags().GetStringArray("exclude")
			cfg := config.Load()
			cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
			ctx, cancel := context.WithTimeout(cmd.Context(), cfg.Timeout)
			defer cancel()
			params := map[string]interface{}{
				"path":    args[0],
				"name":    name,
				"include": include,
				"exclude": exclude,
			}
			var res struct {
				RepoID string `json:"repo_id"`
			}
			if err := cli.Call(ctx, "repos_add", params, &res); err != nil {
				return err
			}
			return output.Print(fmt.Sprintf("repo added: %s", res.RepoID), cfg.OutputFormat)
		},
	}
	add.Flags().String("name", "", "Optional repo name")
	add.Flags().StringArray("include", nil, "Include globs")
	add.Flags().StringArray("exclude", nil, "Exclude globs")

	scan := &cobra.Command{
		Use:   "scan <path|name>",
		Args:  cobra.ExactArgs(1),
		Short: "Scan and index a repo",
		RunE: func(cmd *cobra.Command, args []string) error {
			include, _ := cmd.Flags().GetStringArray("include")
			exclude, _ := cmd.Flags().GetStringArray("exclude")
			watch, _ := cmd.Flags().GetBool("watch")
			debounce, _ := cmd.Flags().GetDuration("debounce")
			cfg := config.Load()
			cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
			ctx, cancel := context.WithTimeout(cmd.Context(), cfg.Timeout)
			defer cancel()
			params := map[string]interface{}{
				"repo_path": args[0],
				"filters": map[string]interface{}{
					"include": include,
					"exclude": exclude,
				},
				"watch":    watch,
				"debounce": debounce.Milliseconds(),
			}
			var res struct {
				JobID        string `json:"job_id"`
				FilesScanned int64  `json:"files_scanned"`
				DocsAdded    int64  `json:"docs_added"`
				Errors       int64  `json:"errors"`
			}
			if err := cli.Call(ctx, "scan_repo", params, &res); err != nil {
				return err
			}
			return output.Print(res, cfg.OutputFormat)
		},
	}
	scan.Flags().StringArray("include", nil, "Include globs")
	scan.Flags().StringArray("exclude", nil, "Exclude globs")
	scan.Flags().Bool("watch", false, "Watch for changes")
	scan.Flags().Duration("debounce", 200*time.Millisecond, "Debounce for watcher events")

	list := &cobra.Command{
		Use:   "list",
		Short: "List repos",
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg := config.Load()
			cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
			ctx, cancel := context.WithTimeout(cmd.Context(), cfg.Timeout)
			defer cancel()
			var res []struct {
				ID   string `json:"id"`
				Name string `json:"name"`
				Path string `json:"path"`
			}
			if err := cli.Call(ctx, "repos_list", map[string]interface{}{}, &res); err != nil {
				return err
			}
			return output.Print(res, cfg.OutputFormat)
		},
	}

	info := &cobra.Command{
		Use:   "info <name|id>",
		Args:  cobra.ExactArgs(1),
		Short: "Show repo info",
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg := config.Load()
			cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
			ctx, cancel := context.WithTimeout(cmd.Context(), cfg.Timeout)
			defer cancel()
			var res map[string]interface{}
			if err := cli.Call(ctx, "repos_info", map[string]interface{}{"id_or_name": args[0]}, &res); err != nil {
				return err
			}
			return output.Print(res, cfg.OutputFormat)
		},
	}

	remove := &cobra.Command{
		Use:   "remove <name|id>",
		Args:  cobra.ExactArgs(1),
		Short: "Unregister a repo",
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg := config.Load()
			cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
			ctx, cancel := context.WithTimeout(cmd.Context(), cfg.Timeout)
			defer cancel()
			var res struct {
				Removed bool `json:"removed"`
			}
			if err := cli.Call(ctx, "repos_remove", map[string]interface{}{"id_or_name": args[0]}, &res); err != nil {
				return err
			}
			return output.Print(fmt.Sprintf("removed: %v", res.Removed), cfg.OutputFormat)
		},
	}

	// default-provider
	dp := &cobra.Command{Use: "default-provider", Short: "Manage repo default AI provider"}
	dpSet := &cobra.Command{Use: "set <name|id> <provider>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx, cancel := context.WithTimeout(cmd.Context(), cfg.Timeout)
		defer cancel()
		var res map[string]interface{}
		if err := cli.Call(ctx, "repos_set_default_provider", map[string]interface{}{"id_or_name": args[0], "provider": args[1]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	dp.AddCommand(dpSet)

	repo.AddCommand(add, scan, list, info, remove, dp)
	return repo
}
