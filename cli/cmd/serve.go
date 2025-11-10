package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

func serveCmd() *cobra.Command {
    serve := &cobra.Command{Use: "serve", Short: "Local services"}
    api := &cobra.Command{Use: "api", RunE: func(cmd *cobra.Command, args []string) error { fmt.Println("serve api (stub)"); return nil }}
    serve.AddCommand(api)
    return serve
}
