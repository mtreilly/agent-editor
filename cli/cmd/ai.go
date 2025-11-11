package cmd

import (
	"context"
	"github.com/agent-editor/agent-editor/cli/internal/config"
	"github.com/agent-editor/agent-editor/cli/internal/output"
	"github.com/agent-editor/agent-editor/cli/internal/rpc"
	"github.com/spf13/cobra"
)

func aiCmd() *cobra.Command {
	ai := &cobra.Command{Use: "ai", Short: "AI operations"}

	run := &cobra.Command{Use: "run <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		provider, _ := cmd.Flags().GetString("provider")
		prompt, _ := cmd.Flags().GetString("prompt")
		anchor, _ := cmd.Flags().GetString("anchor")
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res map[string]interface{}
		if err := cli.Call(ctx, "ai_run", map[string]interface{}{"provider": provider, "doc_id": args[0], "anchor_id": anchor, "prompt": prompt}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	run.Flags().String("provider", "local", "Provider name")
	run.Flags().String("prompt", "", "Prompt text")
	run.Flags().String("anchor", "", "Anchor ID (optional)")

	providers := &cobra.Command{Use: "providers", Short: "Manage AI providers"}
	providersList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res []map[string]interface{}
		if err := cli.Call(ctx, "ai_providers_list", map[string]interface{}{}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	providersEnable := &cobra.Command{Use: "enable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res map[string]interface{}
		if err := cli.Call(ctx, "ai_providers_enable", map[string]interface{}{"name": args[0]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	providersDisable := &cobra.Command{Use: "disable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res map[string]interface{}
		if err := cli.Call(ctx, "ai_providers_disable", map[string]interface{}{"name": args[0]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	providersTest := &cobra.Command{Use: "test <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res map[string]interface{}
		if err := cli.Call(ctx, "ai_provider_test", map[string]interface{}{"name": args[0], "prompt": "hello"}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	// keys subcommands
	keys := &cobra.Command{Use: "key", Short: "Manage provider API keys"}
	keySet := &cobra.Command{Use: "set <name> <key>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res map[string]interface{}
		if err := cli.Call(ctx, "ai_provider_key_set", map[string]interface{}{"name": args[0], "key": args[1]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	keyHas := &cobra.Command{Use: "has <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.Load()
		cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
		ctx := context.Background()
		var res map[string]interface{}
		if err := cli.Call(ctx, "ai_provider_key_get", map[string]interface{}{"name": args[0]}, &res); err != nil {
			return err
		}
		return output.Print(res, cfg.OutputFormat)
	}}
	keys.AddCommand(keySet, keyHas)

	providers.AddCommand(providersList, providersEnable, providersDisable, providersTest, keys)

	traces := &cobra.Command{Use: "traces", Short: "AI traces"}
	tracesList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { return output.Print("not implemented", "text") }}
	traces.AddCommand(tracesList)

	ai.AddCommand(run, providers, traces)
	return ai
}
