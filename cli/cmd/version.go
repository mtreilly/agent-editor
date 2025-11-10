package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

var (
    version   = "dev"
    commit    = "none"
    buildDate = "unknown"
)

func versionCmd() *cobra.Command {
    return &cobra.Command{
        Use:   "version",
        Short: "Show version information",
        RunE: func(cmd *cobra.Command, args []string) error {
            fmt.Printf("agent-editor %s (commit: %s, built: %s)\n", version, commit, buildDate)
            return nil
        },
    }
}

