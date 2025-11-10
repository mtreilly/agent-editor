package cmd

import (
    "context"
    "fmt"

    "agent-editor/cli/internal/config"
    "agent-editor/cli/internal/output"
    "agent-editor/cli/internal/rpc"
    "github.com/spf13/cobra"
)

func docCmd() *cobra.Command {
    doc := &cobra.Command{Use: "doc", Short: "Document operations"}

    create := &cobra.Command{Use: "create <repo> <slug>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error {
        title, _ := cmd.Flags().GetString("title")
        body, _ := cmd.Flags().GetString("body")
        cfg := config.Load()
        cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background()
        params := map[string]interface{}{"repo_id": args[0], "slug": args[1], "title": title, "body": body}
        var res struct{ DocID string `json:"doc_id"` }
        if err := cli.Call(ctx, "docs_create", params, &res); err != nil { return err }
        return output.Print(fmt.Sprintf("doc created: %s", res.DocID), cfg.OutputFormat)
    }}
    create.Flags().String("title", "", "Document title")
    create.Flags().String("body", "", "Initial body text")

    update := &cobra.Command{Use: "update <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        body, _ := cmd.Flags().GetString("body")
        message, _ := cmd.Flags().GetString("message")
        cfg := config.Load()
        cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background()
        params := map[string]interface{}{"doc_id": args[0], "body": body, "message": message}
        var res struct{ VersionID string `json:"version_id"` }
        if err := cli.Call(ctx, "docs_update", params, &res); err != nil { return err }
        return output.Print(fmt.Sprintf("version: %s", res.VersionID), cfg.OutputFormat)
    }}
    update.Flags().String("body", "", "Body text")
    update.Flags().String("message", "", "Commit message")

    get := &cobra.Command{Use: "get <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        content, _ := cmd.Flags().GetBool("content")
        cfg := config.Load()
        cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background()
        params := map[string]interface{}{"doc_id": args[0], "content": content}
        var res map[string]interface{}
        if err := cli.Call(ctx, "docs_get", params, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    get.Flags().Bool("content", false, "Include content body")

    del := &cobra.Command{Use: "delete <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load()
        cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background()
        params := map[string]interface{}{"doc_id": args[0]}
        var res struct{ Deleted bool `json:"deleted"` }
        if err := cli.Call(ctx, "docs_delete", params, &res); err != nil { return err }
        return output.Print(fmt.Sprintf("deleted: %v", res.Deleted), cfg.OutputFormat)
    }}

    search := &cobra.Command{Use: "search <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        repoID, _ := cmd.Flags().GetString("repo")
        limit, _ := cmd.Flags().GetInt("limit")
        offset, _ := cmd.Flags().GetInt("offset")
        cfg := config.Load()
        cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background()
        params := map[string]interface{}{"repo_id": repoID, "query": args[0], "limit": limit, "offset": offset}
        var res []map[string]interface{}
        if err := cli.Call(ctx, "search", params, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    search.Flags().String("repo", "", "Repo scope")
    search.Flags().Int("limit", 50, "Limit")
    search.Flags().Int("offset", 0, "Offset")

    doc.AddCommand(create, update, get, del, search)
    return doc
}
