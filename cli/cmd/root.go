package cmd

import (
    "fmt"
    "os"

    "github.com/spf13/cobra"
    "github.com/spf13/viper"
)

var (
    outputFormat string
    verboseCount int
    debugMode    bool
    yesAll       bool
    dryRun       bool
)

// Execute runs the root command.
func Execute() error { return rootCmd().Execute() }

func rootCmd() *cobra.Command {
    cmd := &cobra.Command{
        Use:   "agent-editor",
        Short: "Local-first Markdown knowledge system",
        PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
            return initConfig()
        },
    }

    // Global flags
    cmd.PersistentFlags().CountVarP(&verboseCount, "verbose", "v", "verbosity (-v, -vv)")
    cmd.PersistentFlags().BoolVar(&debugMode, "debug", false, "debug mode")
    cmd.PersistentFlags().StringVarP(&outputFormat, "output", "o", "text", "output format (text|json|yaml)")
    cmd.PersistentFlags().BoolVar(&yesAll, "yes", false, "assume yes for prompts")
    cmd.PersistentFlags().BoolVar(&dryRun, "dry-run", false, "do not make changes")
    cmd.PersistentFlags().String("server", "", "API server base URL (default: http://127.0.0.1:35678)")
    cmd.PersistentFlags().String("token", "", "API token (optional)")

    _ = viper.BindPFlag("server", cmd.PersistentFlags().Lookup("server"))
    _ = viper.BindPFlag("token", cmd.PersistentFlags().Lookup("token"))
    _ = viper.BindPFlag("output", cmd.PersistentFlags().Lookup("output"))
    _ = viper.BindPFlag("verbose", cmd.PersistentFlags().Lookup("verbose"))
    _ = viper.BindPFlag("debug", cmd.PersistentFlags().Lookup("debug"))

    // Subcommands
    cmd.AddCommand(repoCmd())
    cmd.AddCommand(docCmd())
    cmd.AddCommand(ftsCmd())
    cmd.AddCommand(graphCmd())
    cmd.AddCommand(aiCmd())
    cmd.AddCommand(pluginCmd())
    cmd.AddCommand(serveCmd())
    cmd.AddCommand(exportCmd())
    cmd.AddCommand(configCmd())
    cmd.AddCommand(completionCmd(cmd))
    cmd.AddCommand(versionCmd())

    return cmd
}

func initConfig() error {
    v := viper.GetViper()
    // Env
    v.SetEnvPrefix("AGENT_EDITOR")
    v.AutomaticEnv()

    // Config file (XDG)
    cfgPath := os.Getenv("XDG_CONFIG_HOME")
    if cfgPath == "" {
        home, _ := os.UserHomeDir()
        cfgPath = fmt.Sprintf("%s/.config", home)
    }
    v.SetConfigName("config")
    v.SetConfigType("yaml")
    v.AddConfigPath(fmt.Sprintf("%s/agent-editor", cfgPath))

    _ = v.ReadInConfig() // optional
    return nil
}
