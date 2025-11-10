package output

import (
    "encoding/json"
    "errors"
    "fmt"
    "os"

    "gopkg.in/yaml.v3"
)

func Print(v interface{}, format string) error {
    switch format {
    case "json":
        enc := json.NewEncoder(os.Stdout)
        enc.SetIndent("", "  ")
        return enc.Encode(v)
    case "yaml":
        enc := yaml.NewEncoder(os.Stdout)
        enc.SetIndent(2)
        return enc.Encode(v)
    case "text", "":
        switch vv := v.(type) {
        case string:
            fmt.Println(vv)
            return nil
        case []string:
            for _, s := range vv {
                fmt.Println(s)
            }
            return nil
        default:
            b, _ := json.Marshal(v)
            fmt.Println(string(b))
            return nil
        }
    default:
        return errors.New("unknown output format")
    }
}

