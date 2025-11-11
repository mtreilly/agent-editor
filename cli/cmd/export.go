package cmd

import (
    "bufio"
    "context"
    "encoding/json"
    "fmt"
    "os"
    "strings"

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
        format, _ := cmd.Flags().GetString("format")
        format = strings.ToLower(format)
        if format == "" { format = "json" }
        if format != "json" && format != "jsonl" {
            return fmt.Errorf("invalid --format %s (expected json|jsonl)", format)
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
        var res []map[string]interface{}
        if err := cli.Call(ctx, "export_docs", params, &res); err != nil { return err }

        if outFile != "" {
            switch format {
            case "json":
                data, err := json.MarshalIndent(res, "", "  ")
                if err != nil { return err }
                if err := os.WriteFile(outFile, data, 0o644); err != nil { return err }
            case "jsonl":
                f, err := os.Create(outFile)
                if err != nil { return err }
                defer f.Close()
                w := bufio.NewWriter(f)
                for _, row := range res {
                    line, err := json.Marshal(row)
                    if err != nil { return err }
                    if _, err := w.Write(append(line, '\n')); err != nil { return err }
                }
                if err := w.Flush(); err != nil { return err }
            }
            return output.Print(fmt.Sprintf("exported %d docs to %s (%s)", len(res), outFile, format), cfg.OutputFormat)
        }
        return output.Print(res, cfg.OutputFormat)
    }}
    docs.Flags().String("repo", "", "Repo ID to filter (default: all repos)")
    docs.Flags().Bool("include-deleted", false, "Include docs marked as deleted")
    docs.Flags().String("out", "", "Write JSON export to file")
    docs.Flags().String("format", "json", "Output format when using --out (json|jsonl)")

    db := &cobra.Command{Use: "db", RunE: func(cmd *cobra.Command, args []string) error {
        outFile, _ := cmd.Flags().GetString("out")
        if outFile == "" { return fmt.Errorf("--out is required") }
        cfg := config.Load()
        cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background()
        var res map[string]interface{}
        if err := cli.Call(ctx, "export_db", map[string]interface{}{"out_path": outFile}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    db.Flags().String("out", "", "Destination path for SQLite backup")

    export.AddCommand(docs, db)
    return export
}
