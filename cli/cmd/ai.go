package cmd

import (
    "context"
    "agent-editor/cli/internal/config"
    "agent-editor/cli/internal/output"
    "agent-editor/cli/internal/rpc"
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
        if err := cli.Call(ctx, "ai_run", map[string]interface{}{"provider": provider, "doc_id": args[0], "anchor_id": anchor, "prompt": prompt}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    run.Flags().String("provider", "local", "Provider name")
    run.Flags().String("prompt", "", "Prompt text")
    run.Flags().String("anchor", "", "Anchor ID (optional)")

    providers := &cobra.Command{Use: "providers", Short: "Manage AI providers"}
    providersList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { return output.Print("not implemented", "text") }}
    providersEnable := &cobra.Command{Use: "enable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { return output.Print("not implemented", "text") }}
    providersDisable := &cobra.Command{Use: "disable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { return output.Print("not implemented", "text") }}
    providers.AddCommand(providersList, providersEnable, providersDisable)

    traces := &cobra.Command{Use: "traces", Short: "AI traces"}
    tracesList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { return output.Print("not implemented", "text") }}
    traces.AddCommand(tracesList)

    ai.AddCommand(run, providers, traces)
    return ai
}
