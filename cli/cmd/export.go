package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

func exportCmd() *cobra.Command {
    export := &cobra.Command{Use: "export", Short: "Export data"}
    docs := &cobra.Command{Use: "docs", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("export docs (stub)"); return nil }}
    db := &cobra.Command{Use: "db", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("export db (stub)"); return nil }}
    export.AddCommand(docs, db)
    return export
}
