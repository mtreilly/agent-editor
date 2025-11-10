package cmd

import (
    "fmt"
    "os"

    "github.com/spf13/cobra"
)

func completionCmd(root *cobra.Command) *cobra.Command {
    cmd := &cobra.Command{Use: "completion", Short: "Shell completions"}
    generate := &cobra.Command{
        Use:   "generate <bash|zsh|fish>",
        Args:  cobra.ExactArgs(1),
        Short: "Generate completion script",
        RunE: func(cmd *cobra.Command, args []string) error {
            switch args[0] {
            case "bash":
                return root.GenBashCompletion(os.Stdout)
            case "zsh":
                return root.GenZshCompletion(os.Stdout)
            case "fish":
                return root.GenFishCompletion(os.Stdout, true)
            default:
                return fmt.Errorf("unknown shell: %s", args[0])
            }
        },
    }
    cmd.AddCommand(generate)
    return cmd
}
