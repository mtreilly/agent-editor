package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

func configCmd() *cobra.Command {
    config := &cobra.Command{Use: "config", Short: "CLI configuration"}
    init := &cobra.Command{Use: "init", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("config init (stub)"); return nil }}
    get := &cobra.Command{Use: "get <key>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("config get (stub)", args[0]); return nil }}
    set := &cobra.Command{Use: "set <key> <value>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("config set (stub)", args); return nil }}
    path := &cobra.Command{Use: "path", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("config path (stub)"); return nil }}
    config.AddCommand(init, get, set, path)
    return config
}
