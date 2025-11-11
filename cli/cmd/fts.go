package cmd

import (
	"fmt"
	"sort"
	"time"

	"github.com/agent-editor/agent-editor/cli/internal/config"
	"github.com/agent-editor/agent-editor/cli/internal/output"
	"github.com/agent-editor/agent-editor/cli/internal/rpc"
	"github.com/spf13/cobra"
)

func ftsCmd() *cobra.Command {
	fts := &cobra.Command{Use: "fts", Short: "Full-text search ops"}

	query := &cobra.Command{Use: "query <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		var res []map[string]interface{}
		params := map[string]interface{}{"query": args[0], "limit": 50, "offset": 0}
		if err := cli.Call(cmd.Context(), "search", params, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	reindex := &cobra.Command{Use: "reindex", RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Println("fts reindex (not implemented)")
		return nil
	}}
	stats := &cobra.Command{Use: "stats", RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		var res map[string]interface{}
		if err := cli.Call(cmd.Context(), "fts_stats", map[string]interface{}{}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}

	bench := &cobra.Command{Use: "bench", Short: "Benchmark search latency", RunE: func(cmd *cobra.Command, args []string) error {
		q, _ := cmd.Flags().GetString("query")
		n, _ := cmd.Flags().GetInt("n")
		repo, _ := cmd.Flags().GetString("repo")
		if q == "" {
			q = "the"
		}
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		type Hit = map[string]interface{}
		durs := make([]time.Duration, 0, n)
		for i := 0; i < n; i++ {
			start := time.Now()
			var res []Hit
			params := map[string]interface{}{"query": q, "limit": 50, "offset": 0}
			if repo != "" {
				params["repo_id"] = repo
			}
			if err := cli.Call(cmd.Context(), "search", params, &res); err != nil {
				return err
			}
			durs = append(durs, time.Since(start))
		}
		var sum time.Duration
		for _, d := range durs {
			sum += d
		}
		avg := sum / time.Duration(len(durs))
		// percentiles
		sorted := append([]time.Duration(nil), durs...)
		sort.Slice(sorted, func(i, j int) bool { return sorted[i] < sorted[j] })
		pick := func(p float64) time.Duration {
			if len(sorted) == 0 {
				return 0
			}
			idx := int(p*float64(len(sorted)-1) + 0.5)
			if idx < 0 {
				idx = 0
			}
			if idx >= len(sorted) {
				idx = len(sorted) - 1
			}
			return sorted[idx]
		}
		p50 := pick(0.50)
		p95 := pick(0.95)
		p99 := pick(0.99)
		return output.Print(map[string]any{
			"runs":   n,
			"avg_ms": float64(avg.Microseconds()) / 1000.0,
			"p50_ms": float64(p50.Microseconds()) / 1000.0,
			"p95_ms": float64(p95.Microseconds()) / 1000.0,
			"p99_ms": float64(p99.Microseconds()) / 1000.0,
		}, cfg.OutputFormat)
	}}
	bench.Flags().String("query", "the", "Query to test")
	bench.Flags().Int("n", 25, "Number of runs")
	bench.Flags().String("repo", "", "Repo scope (optional)")

	fts.AddCommand(query, reindex, stats, bench)
	return fts
}
