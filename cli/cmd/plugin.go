package cmd

import (
    "bufio"
    "context"
    "errors"
    "fmt"
    "os"
    "strings"
    "time"

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

    callCore := &cobra.Command{Use: "call-core <name> <line>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "plugins_call_core", map[string]interface{}{"name": args[0], "line": args[1]}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}

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

    coreList := &cobra.Command{Use: "core-list", RunE: func(cmd *cobra.Command, args []string) error {
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res []map[string]interface{}
        if err := cli.Call(ctx, "plugins_core_list", map[string]interface{}{}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}

    events := &cobra.Command{Use: "events", Short: "Plugin events"}
    eventsTail := &cobra.Command{Use: "tail", Short: "Tail plugin log events", RunE: func(cmd *cobra.Command, args []string) error {
        file, _ := cmd.Flags().GetString("file")
        follow, _ := cmd.Flags().GetBool("follow")
        all, _ := cmd.Flags().GetBool("all")
        fromBeginning, _ := cmd.Flags().GetBool("from-beginning")
        if file == "" { file = ".sidecar.log" }
        f, err := os.Open(file)
        if err != nil { return err }
        defer f.Close()

        // Optionally seek to end
        if !fromBeginning {
            if _, err := f.Seek(0, os.SEEK_END); err != nil { return err }
        }

        reader := bufio.NewReader(f)
        for {
            line, err := reader.ReadString('\n')
            if err != nil {
                if errors.Is(err, os.ErrClosed) { return nil }
                // If following, wait and retry; else EOF means done
                if !follow {
                    if len(strings.TrimSpace(line)) > 0 {
                        if all || strings.Contains(line, "][plugin:") { fmt.Print(line) }
                    }
                    return nil
                }
                time.Sleep(250 * time.Millisecond)
                continue
            }
            if all || strings.Contains(line, "][plugin:") {
                fmt.Print(line)
            }
        }
    }}
    eventsTail.Flags().String("file", ".sidecar.log", "Path to sidecar log file")
    eventsTail.Flags().Bool("follow", true, "Keep streaming new lines (like tail -f)")
    eventsTail.Flags().Bool("all", false, "Show all lines (not only [plugin:*] prefixed)")
    eventsTail.Flags().Bool("from-beginning", false, "Start reading from beginning (default seeks to end)")
    events.AddCommand(eventsTail)

    plugin.AddCommand(install, list, info, remove, enable, disable, call, callCore, startCore, stopCore, coreList, events)
    // perms subcommand
    perms := &cobra.Command{Use: "perms", Short: "Manage plugin permissions"}
    permsSet := &cobra.Command{Use: "set <name> --json <perms>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error {
        jsonStr, _ := cmd.Flags().GetString("json")
        if jsonStr == "" { jsonStr = "{}" }
        cfg := config.Load(); cli := rpc.New(cfg.ServerURL, cfg.APIToken, cfg.Timeout)
        ctx := context.Background(); var res map[string]interface{}
        if err := cli.Call(ctx, "plugins_upsert", map[string]interface{}{"name": args[0], "permissions": jsonStr}, &res); err != nil { return err }
        return output.Print(res, cfg.OutputFormat)
    }}
    permsSet.Flags().String("json", "", "Permissions JSON, e.g. {'core':{'call':true},'fs':{'read':true}}")
    perms.AddCommand(permsSet)
    plugin.AddCommand(perms)
    return plugin
}
