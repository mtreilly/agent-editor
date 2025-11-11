package cmd

import (
	"context"
	"github.com/agent-editor/agent-editor/cli/internal/config"
	"github.com/agent-editor/agent-editor/cli/internal/output"
	"github.com/agent-editor/agent-editor/cli/internal/rpc"
	"github.com/spf13/cobra"
)

func graphCmd() *cobra.Command {
	graph := &cobra.Command{Use: "graph", Short: "Link graph operations"}

	neighbors := &cobra.Command{Use: "neighbors <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		depth, _ := cmd.Flags().GetInt("depth")
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res []map[string]interface{}
		if err := cli.Call(ctx, "graph_neighbors", map[string]interface{}{"doc_id": args[0], "depth": depth}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	neighbors.Flags().Int("depth", 1, "Depth (1..2)")

	backlinks := &cobra.Command{Use: "backlinks <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res []map[string]interface{}
		if err := cli.Call(ctx, "graph_backlinks", map[string]interface{}{"doc_id": args[0]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}

	// Placeholders for future
	path := &cobra.Command{Use: "path <start-id> <end-id>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res []string
		if err := cli.Call(ctx, "graph_path", map[string]interface{}{"start_id": args[0], "end_id": args[1]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	related := &cobra.Command{Use: "related <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res []map[string]interface{}
		if err := cli.Call(ctx, "graph_related", map[string]interface{}{"doc_id": args[0]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}

	graph.AddCommand(neighbors, backlinks, path, related)
	return graph
}
