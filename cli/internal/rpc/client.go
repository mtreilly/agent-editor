package rpc

import (
    "bytes"
    "context"
    "encoding/json"
    "fmt"
    "net/http"
    "time"
)

type Client struct {
    BaseURL string
    Token   string
    http    *http.Client
}

type request struct {
    JSONRPC string      `json:"jsonrpc"`
    ID      string      `json:"id"`
    Method  string      `json:"method"`
    Params  interface{} `json:"params,omitempty"`
}

type Response struct {
    JSONRPC string           `json:"jsonrpc"`
    ID      string           `json:"id"`
    Result  *json.RawMessage `json:"result,omitempty"`
    Error   *RPCError        `json:"error,omitempty"`
}

type RPCError struct {
    Code    int         `json:"code"`
    Message string      `json:"message"`
    Data    interface{} `json:"data,omitempty"`
}

func New(baseURL, token string, timeout time.Duration) *Client {
    return &Client{
        BaseURL: baseURL,
        Token:   token,
        http:    &http.Client{Timeout: timeout},
    }
}

func (c *Client) Call(ctx context.Context, method string, params interface{}, out interface{}) error {
    reqBody := request{JSONRPC: "2.0", ID: fmt.Sprintf("%d", time.Now().UnixNano()), Method: method, Params: params}
    data, _ := json.Marshal(reqBody)
    req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.BaseURL+"/rpc", bytes.NewReader(data))
    if err != nil {
        return err
    }
    req.Header.Set("Content-Type", "application/json")
    if c.Token != "" {
        req.Header.Set("Authorization", "Bearer "+c.Token)
    }
    resp, err := c.http.Do(req)
    if err != nil {
        return err
    }
    defer resp.Body.Close()
    if resp.StatusCode < 200 || resp.StatusCode >= 300 {
        return fmt.Errorf("rpc http status %d", resp.StatusCode)
    }
    var r Response
    if err := json.NewDecoder(resp.Body).Decode(&r); err != nil {
        return err
    }
    if r.Error != nil {
        return fmt.Errorf("rpc error %d: %s", r.Error.Code, r.Error.Message)
    }
    if out != nil && r.Result != nil {
        return json.Unmarshal(*r.Result, out)
    }
    return nil
}

