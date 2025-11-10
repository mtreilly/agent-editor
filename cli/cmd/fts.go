package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

func ftsCmd() *cobra.Command {
    fts := &cobra.Command{Use: "fts", Short: "Full-text search ops"}

    query := &cobra.Command{Use: "query <query>", Args: cobra.ExactArgs(1), RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("fts query (stub)", args[0]); return nil }}
    reindex := &cobra.Command{Use: "reindex", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("fts reindex (stub)"); return nil }}
    stats := &cobra.Command{Use: "stats", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("fts stats (stub)"); return nil }}

    fts.AddCommand(query, reindex, stats)
    return fts
}
