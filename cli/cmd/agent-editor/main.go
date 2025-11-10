package main

import (
    "context"
    "fmt"
    "os"
    "time"

    "github.com/spf13/cobra"
    "github.com/spf13/viper"

    cfgpkg "github.com/agent-editor/agent-editor/cli/internal/config"
    out "github.com/agent-editor/agent-editor/cli/internal/output"
    rpc "github.com/agent-editor/agent-editor/cli/internal/rpc"
)

var (
    outputFormat string
    verbose      int
    debug        bool
    configPath   string
)

func main() {
    rootCmd := &cobra.Command{
        Use:   "agent-editor",
        Short: "Local-first Markdown knowledge system",
        PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
            if configPath != "" {
                viper.SetConfigFile(configPath)
                _ = viper.ReadInConfig()
            }
            viper.SetDefault("server", "http://127.0.0.1:35678")
            viper.SetDefault("timeout", 30)
            viper.Set("output", outputFormat)
            viper.Set("verbose", verbose)
            viper.Set("debug", debug)
            return nil
        },
    }

    rootCmd.PersistentFlags().CountVarP(&verbose, "verbose", "v", "verbose output (-v, -vv)")
    rootCmd.PersistentFlags().BoolVar(&debug, "debug", false, "debug mode")
    rootCmd.PersistentFlags().StringVarP(&outputFormat, "output", "o", "text", "output format (text|json|yaml)")
    rootCmd.PersistentFlags().StringVarP(&configPath, "config", "c", "", "config file path")

    // Subcommands
    rootCmd.AddCommand(repoCmd())
    rootCmd.AddCommand(docCmd())
    rootCmd.AddCommand(graphCmd())
    rootCmd.AddCommand(ftsCmd())
    rootCmd.AddCommand(aiCmd())
    rootCmd.AddCommand(serveCmd())
    rootCmd.AddCommand(versionCmd())

    if err := rootCmd.Execute(); err != nil {
        fmt.Fprintln(os.Stderr, err)
        os.Exit(1)
    }
}

func clientFromConfig() (*rpc.Client, *cfgpkg.Config) {
    cfg := cfgpkg.Load()
    return rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout), cfg
}

// ---- repo ----
func repoCmd() *cobra.Command {
    cmd := &cobra.Command{Use: "repo", Short: "Manage repositories"}
    var include, exclude []string
    add := &cobra.Command{Use: "add <path>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        type resp struct{ RepoID string `json:"repo_id"` }
        var outResp resp
        err := c.Call(context.TODO(), "repos_add", map[string]any{"path": args[0], "name": cmd.Flag("name").Value.String(), "include": include, "exclude": exclude}, &outResp)
        if err != nil { return err }
        return out.Print(outResp, outputFormat)
    }}
    add.Flags().String("name", "", "optional repo name")
    add.Flags().StringArrayVar(&include, "include", nil, "include globs")
    add.Flags().StringArrayVar(&exclude, "exclude", nil, "exclude globs")

    var watch bool
    var debounce time.Duration
    scan := &cobra.Command{Use: "scan <path|name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        type resp struct{ JobID string `json:"job_id"`; FilesScanned int64 `json:"files_scanned"`; DocsAdded int64 `json:"docs_added"`; Errors int64 `json:"errors"` }
        var outResp resp
        params := map[string]any{"repo_path": args[0], "filters": map[string]any{"include": include, "exclude": exclude}, "watch": watch, "debounce": int(debounce.Milliseconds())}
        if err := c.Call(context.TODO(), "scan_repo", params, &outResp); err != nil { return err }
        return out.Print(outResp, outputFormat)
    }}
    scan.Flags().StringArrayVar(&include, "include", nil, "include globs")
    scan.Flags().StringArrayVar(&exclude, "exclude", nil, "exclude globs")
    scan.Flags().BoolVar(&watch, "watch", false, "watch for changes")
    scan.Flags().DurationVar(&debounce, "debounce", 200*time.Millisecond, "watch debounce interval")

    list := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res []map[string]any
        if err := c.Call(context.TODO(), "repos_list", nil, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    info := &cobra.Command{Use: "info <name|id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res map[string]any
        if err := c.Call(context.TODO(), "repos_info", map[string]any{"id_or_name": args[0]}, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    remove := &cobra.Command{Use: "remove <name|id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res map[string]any
        if err := c.Call(context.TODO(), "repos_remove", map[string]any{"id_or_name": args[0]}, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    cmd.AddCommand(add, scan, list, info, remove)
    return cmd
}

// ---- doc ----
func docCmd() *cobra.Command {
    cmd := &cobra.Command{Use: "doc", Short: "Document operations"}
    search := &cobra.Command{Use: "search <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res []map[string]any
        if err := c.Call(context.TODO(), "search", map[string]any{"query": args[0], "limit": 50, "offset": 0}, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    cmd.AddCommand(search)
    return cmd
}

// ---- graph ----
func graphCmd() *cobra.Command {
    cmd := &cobra.Command{Use: "graph", Short: "Link graph operations"}
    neighbors := &cobra.Command{Use: "neighbors <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res []map[string]any
        if err := c.Call(context.TODO(), "graph_neighbors", map[string]any{"doc_id": args[0], "depth": 1}, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    backlinks := &cobra.Command{Use: "backlinks <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res []map[string]any
        if err := c.Call(context.TODO(), "graph_backlinks", map[string]any{"doc_id": args[0]}, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    cmd.AddCommand(neighbors, backlinks)
    return cmd
}

// ---- fts ----
func ftsCmd() *cobra.Command {
    cmd := &cobra.Command{Use: "fts", Short: "Full-text search ops"}
    query := &cobra.Command{Use: "query <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res []map[string]any
        if err := c.Call(context.TODO(), "search", map[string]any{"query": args[0], "limit": 50, "offset": 0}, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    cmd.AddCommand(query)
    return cmd
}

// ---- ai ----
func aiCmd() *cobra.Command {
    cmd := &cobra.Command{Use: "ai", Short: "AI operations"}
    run := &cobra.Command{Use: "run <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        provider, _ := cmd.Flags().GetString("provider")
        prompt, _ := cmd.Flags().GetString("prompt")
        var res map[string]any
        if err := c.Call(context.TODO(), "ai_run", map[string]any{"provider": provider, "doc_id": args[0], "prompt": prompt}, &res); err != nil { return err }
        return out.Print(res, outputFormat)
    }}
    run.Flags().String("provider", "local", "provider name")
    run.Flags().String("prompt", "Explain this", "prompt text")
    cmd.AddCommand(run)
    return cmd
}

// ---- serve ----
func serveCmd() *cobra.Command {
    cmd := &cobra.Command{Use: "serve", Short: "Serve JSON-RPC bridge"}
    api := &cobra.Command{Use: "api", RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        var res any
        return c.Call(context.TODO(), "serve_api_start", map[string]any{"port": 35678}, &res)
    }}
    cmd.AddCommand(api)
    return cmd
}

// ---- version ----
func versionCmd() *cobra.Command {
    return &cobra.Command{Use: "version", RunE: func(cmd *cobra.Command, args []string) error {
        return out.Print(map[string]string{"version": "0.0.0"}, outputFormat)
    }}
}

