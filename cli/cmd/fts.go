package cmd

import (
    "fmt"
    "time"

    "agent-editor/cli/internal/config"
    "agent-editor/cli/internal/output"
    "agent-editor/cli/internal/rpc"
    "github.com/spf13/cobra"
)

func ftsCmd() *cobra.Command {
    fts := &cobra.Command{Use: "fts", Short: "Full-text search ops"}

    query := &cobra.Command{Use: "query <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("fts query (stub)", args[0]); return nil }}
    reindex := &cobra.Command{Use: "reindex", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("fts reindex (stub)"); return nil }}
    stats := &cobra.Command{Use: "stats", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("fts stats (stub)"); return nil }}

    bench := &cobra.Command{Use: "bench", Short: "Benchmark search latency", RunE: func(cmd *cobra.Command, args []string) error {
        q, _ := cmd.Flags().GetString("query")
        n, _ := cmd.Flags().GetInt("n")
        repo, _ := cmd.Flags().GetString("repo")
        if q == "" { q = "the" }
        cfg := config.Load()
        cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        type Hit = map[string]interface{}
        durs := make([]time.Duration, 0, n)
        for i := 0; i < n; i++ {
            start := time.Now()
            var res []Hit
            params := map[string]interface{}{"query": q, "limit": 50, "offset": 0}
            if repo != "" { params["repo_id"] = repo }
            if err := cli.Call(cmd.Context(), "search", params, &res); err != nil { return err }
            durs = append(durs, time.Since(start))
        }
        var sum time.Duration
        for _, d := range durs { sum += d }
        avg := sum / time.Duration(len(durs))
        return output.Print(map[string]any{"runs": n, "avg_ms": float64(avg.Microseconds())/1000.0}, cfg.OutputFormat)
    }}
    bench.Flags().String("query", "the", "Query to test")
    bench.Flags().Int("n", 25, "Number of runs")
    bench.Flags().String("repo", "", "Repo scope (optional)")

    fts.AddCommand(query, reindex, stats, bench)
    return fts
}
