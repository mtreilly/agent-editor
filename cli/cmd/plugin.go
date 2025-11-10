package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

func pluginCmd() *cobra.Command {
    plugin := &cobra.Command{Use: "plugin", Short: "Plugin management"}

    install := &cobra.Command{Use: "install <name|path>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin install (stub)", args[0]); return nil }}
    list := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin list (stub)"); return nil }}
    info := &cobra.Command{Use: "info <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin info (stub)", args[0]); return nil }}
    remove := &cobra.Command{Use: "remove <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin remove (stub)", args[0]); return nil }}
    enable := &cobra.Command{Use: "enable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin enable (stub)", args[0]); return nil }}
    disable := &cobra.Command{Use: "disable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin disable (stub)", args[0]); return nil }}
    call := &cobra.Command{Use: "call <name> <method>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin call (stub)", args); return nil }}

    events := &cobra.Command{Use: "events", Short: "Plugin events"}
    eventsTail := &cobra.Command{Use: "tail", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("plugin events tail (stub)"); return nil }}
    events.AddCommand(eventsTail)

    plugin.AddCommand(install, list, info, remove, enable, disable, call, events)
    return plugin
}
