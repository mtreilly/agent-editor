package cmd

import (
	"fmt"

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
			if repo != "" && newRepo != "" {
				return fmt.Errorf("--repo and --new-repo are mutually exclusive")
			}
			cfg := config.Load()
			cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
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
			var res map[string]interface{}
			if err := cli.Call(cmd.Context(), "import_docs", payload, &res); err != nil {
				return err
			}
			return output.Print(res, cfg.OutputFormat)
		},
	}
	docs.Flags().String("repo", "", "Existing repo to import into")
	docs.Flags().String("new-repo", "", "Create a new repo for import")
	docs.Flags().Bool("dry-run", false, "Validate without writing to DB")
	docs.Flags().String("merge-strategy", "keep", "Conflict strategy: keep|overwrite")

	importRoot.AddCommand(docs)
	return importRoot
}
