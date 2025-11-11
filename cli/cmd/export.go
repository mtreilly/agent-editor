package cmd

import (
    "context"
    "encoding/json"
    "fmt"
    "os"

    "agent-editor/cli/internal/config"
    "agent-editor/cli/internal/output"
    "agent-editor/cli/internal/rpc"
    "github.com/spf13/cobra"
)

func exportCmd() *cobra.Command {
    export := &cobra.Command{Use: "export", Short: "Export data"}

    docs := &cobra.Command{Use: "docs", RunE: func(cmd *cobra.Command, args []string) error {
        repo, _ := cmd.Flags().GetString("repo")
        includeDeleted, _ := cmd.Flags().GetBool("include-deleted")
        outFile, _ := cmd.Flags().GetString("out")

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
        var res []map[string]interface{}
        if err := cli.Call(ctx, "export_docs", params, &res); err != nil { return err }

        if outFile != "" {
            data, err := json.MarshalIndent(res, "", "  ")
            if err != nil { return err }
            if err := os.WriteFile(outFile, data, 0o644); err != nil { return err }
            return output.Print(fmt.Sprintf("exported %d docs to %s", len(res), outFile), cfg.OutputFormat)
        }
        return output.Print(res, cfg.OutputFormat)
    }}
    docs.Flags().String("repo", "", "Repo ID to filter (default: all repos)")
    docs.Flags().Bool("include-deleted", false, "Include docs marked as deleted")
    docs.Flags().String("out", "", "Write JSON export to file")

    db := &cobra.Command{Use: "db", RunE: func(cmd *cobra.Command, args []string) error {
        fmt.Println("export db (stub)")
        return nil
    }}

    export.AddCommand(docs, db)
    return export
}
