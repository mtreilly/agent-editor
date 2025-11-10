package config

import (
    "time"

    "github.com/spf13/viper"
)

type Config struct {
    ServerURL     string
    APIToken      string
    Timeout       time.Duration
    OutputFormat  string
    VerboseLevels int
    Debug         bool
}

func Load() *Config {
    v := viper.GetViper()
    // Defaults
    if !v.IsSet("server") {
        v.Set("server", "http://127.0.0.1:35678")
    }
    if !v.IsSet("timeout") {
        v.Set("timeout", 30)
    }
    cfg := &Config{
        ServerURL:     v.GetString("server"),
        APIToken:      v.GetString("token"),
        Timeout:       time.Duration(v.GetInt("timeout")) * time.Second,
        OutputFormat:  v.GetString("output"),
        VerboseLevels: v.GetInt("verbose"),
        Debug:         v.GetBool("debug"),
    }
    return cfg
}

