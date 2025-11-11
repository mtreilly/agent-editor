package cmd

import (
    "context"
    "agent-editor/cli/internal/config"
    "agent-editor/cli/internal/output"
    "agent-editor/cli/internal/rpc"
    "github.com/spf13/cobra"
)

func settingsCmd() *cobra.Command {
    settings := &cobra.Command{Use: "settings", Short: "App settings"}

    dp := &cobra.Command{Use: "default-provider", Short: "Global default AI provider"}
    dpGet := &cobra.Command{Use: "get", RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "app_settings_get", map[string]interface{}{"key": "default_provider"}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    dpSet := &cobra.Command{Use: "set <provider>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "app_settings_set", map[string]interface{}{"key": "default_provider", "value": args[0]}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    dp.AddCommand(dpGet, dpSet)

    settings.AddCommand(dp)
    return settings
}

