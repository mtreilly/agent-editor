package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

func graphCmd() *cobra.Command {
    graph := &cobra.Command{Use: "graph", Short: "Link graph operations"}

    neighbors := &cobra.Command{Use: "neighbors <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("graph neighbors (stub)", args[0]); return nil }}
    path := &cobra.Command{Use: "path <start-id> <end-id>", Args: cobra.ExactArgs(2), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("graph path (stub)", args); return nil }}
    related := &cobra.Command{Use: "related <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("graph related (stub)", args[0]); return nil }}
    backlinks := &cobra.Command{Use: "backlinks <doc-id>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("graph backlinks (stub)", args[0]); return nil }}

    graph.AddCommand(neighbors, path, related, backlinks)
    return graph
}
