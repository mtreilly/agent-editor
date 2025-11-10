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
    "sort"
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
    // bench: run many search calls and report latency stats
    bench := &cobra.Command{Use: "bench", Short: "Benchmark FTS query latency", RunE: func(cmd *cobra.Command, args []string) error {
        c, _ := clientFromConfig()
        q, _ := cmd.Flags().GetString("query")
        n, _ := cmd.Flags().GetInt("n")
        warm, _ := cmd.Flags().GetInt("warmup")
        repo, _ := cmd.Flags().GetString("repo")
        if q == "" { return fmt.Errorf("--query is required") }
        // warmup
        for i := 0; i < warm; i++ {
            var tmp []map[string]any
            _ = c.Call(context.TODO(), "search", map[string]any{"query": q, "repo_id": repo, "limit": 5, "offset": 0}, &tmp)
        }
        // measure
        durs := make([]time.Duration, 0, n)
        errCount := 0
        for i := 0; i < n; i++ {
            start := time.Now()
            var tmp []map[string]any
            if err := c.Call(context.TODO(), "search", map[string]any{"query": q, "repo_id": repo, "limit": 5, "offset": 0}, &tmp); err != nil { errCount++ }
            durs = append(durs, time.Since(start))
        }
        // stats
        ms := make([]float64, 0, len(durs))
        var sum float64
        var min, max float64
        for i, d := range durs {
            v := float64(d.Microseconds()) / 1000.0
            ms = append(ms, v)
            sum += v
            if i == 0 || v < min { min = v }
            if i == 0 || v > max { max = v }
        }
        sort.Float64s(ms)
        mean := 0.0
        if len(ms) > 0 { mean = sum / float64(len(ms)) }
        p := func(pct float64) float64 { if len(ms) == 0 { return 0 }; idx := int(pct*float64(len(ms)-1) + 0.5); if idx < 0 { idx = 0 }; if idx >= len(ms) { idx = len(ms)-1 }; return ms[idx] }
        res := map[string]any{
            "runs": n,
            "errors": errCount,
            "mean_ms": mean,
            "min_ms": min,
            "p50_ms": p(0.50),
            "p95_ms": p(0.95),
            "p99_ms": p(0.99),
            "max_ms": max,
            "query": q,
            "repo": repo,
        }
        return out.Print(res, outputFormat)
    }}
    bench.Flags().String("query", "", "query to run")
    bench.Flags().Int("n", 50, "number of runs")
    bench.Flags().Int("warmup", 5, "warmup runs")
    bench.Flags().String("repo", "", "optional repo id")
    cmd.AddCommand(query, bench)
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
