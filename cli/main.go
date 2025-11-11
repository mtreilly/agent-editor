package main

import (
	"fmt"
	"github.com/agent-editor/agent-editor/cli/cmd"
	"os"
)

func main() {
	if err := cmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
