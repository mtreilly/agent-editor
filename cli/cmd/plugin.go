package cmd

import (
    "context"
    "agent-editor/cli/internal/config"
    "agent-editor/cli/internal/output"
    "agent-editor/cli/internal/rpc"
    "github.com/spf13/cobra"
)

func pluginCmd() *cobra.Command {
    plugin := &cobra.Command{Use: "plugin", Short: "Plugin management"}

    install := &cobra.Command{Use: "install <name|path>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { return output.Print("not implemented", "text") }}
    list := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res []map[string]interface{}
        if err := cli.Call(ctx, "plugins_list", map[string]interface{}{}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    info := &cobra.Command{Use: "info <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "plugins_info", map[string]interface{}{"name": args[0]}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    remove := &cobra.Command{Use: "remove <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "plugins_remove", map[string]interface{}{"name": args[0]}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    enable := &cobra.Command{Use: "enable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "plugins_enable", map[string]interface{}{"name": args[0]}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    disable := &cobra.Command{Use: "disable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "plugins_disable", map[string]interface{}{"name": args[0]}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    call := &cobra.Command{Use: "call <name> <method>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { return output.Print("not implemented", "text") }}

    startCore := &cobra.Command{Use: "start-core <name> --exec <path> [-- <args>...]", Args: cobra.MinimumNArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        execPath, _ := cmd.Flags().GetString("exec")
        if execPath == "" { return fmt.Errorf("--exec is required") }
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        params := map[string]interface{}{"name": args[0], "exec": execPath}
        if len(args) > 1 { params["args"] = args[1:] }
        if err := cli.Call(ctx, "plugins_spawn_core", params, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    startCore.Flags().String("exec", "", "Executable path for core plugin")

    stopCore := &cobra.Command{Use: "stop-core <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "plugins_shutdown_core", map[string]interface{}{"name": args[0]}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}

    events := &cobra.Command{Use: "events", Short: "Plugin events"}
    eventsTail := &cobra.Command{Use: "tail", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin events tail (stub)"); return nil }}
    events.AddCommand(eventsTail)

    plugin.AddCommand(install, list, info, remove, enable, disable, call, startCore, stopCore, events)
    return plugin
}
