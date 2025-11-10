package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

func aiCmd() *cobra.Command {
    ai := &cobra.Command{Use: "ai", Short: "AI operations"}

    run := &cobra.Command{Use: "run <doc-id|slug>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("ai run (stub)", args[0]); return nil }}

    providers := &cobra.Command{Use: "providers", Short: "Manage AI providers"}
    providersList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("ai providers list (stub)"); return nil }}
    providersEnable := &cobra.Command{Use: "enable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("ai providers enable (stub)", args[0]); return nil }}
    providersDisable := &cobra.Command{Use: "disable <name>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("ai providers disable (stub)", args[0]); return nil }}
    providers.AddCommand(providersList, providersEnable, providersDisable)

    traces := &cobra.Command{Use: "traces", Short: "AI traces"}
    tracesList := &cobra.Command{Use: "list", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("ai traces list (stub)"); return nil }}
    traces.AddCommand(tracesList)

    ai.AddCommand(run, providers, traces)
    return ai
}
