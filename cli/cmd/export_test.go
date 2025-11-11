package cmd

import (
	"archive/tar"
	"encoding/json"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestWriteDocsTar(t *testing.T) {
	dir := t.TempDir()
	tarPath := filepath.Join(dir, "docs.tar")
	docs := []map[string]interface{}{
		{"id": "d1", "slug": "one", "title": "One", "body": "# One", "versions": []interface{}{map[string]interface{}{"id": "v1", "hash": "h1"}}},
		{"id": "d2", "slug": "two", "title": "Two", "body": "# Two"},
	}
	if err := writeDocsTar(tarPath, docs); err != nil {
		t.Fatalf("writeDocsTar error: %v", err)
	}
	f, err := os.Open(tarPath)
	if err != nil {
		t.Fatalf("open tar: %v", err)
	}
	defer f.Close()
	tr := tar.NewReader(f)
	seenDocs := false
	seenMeta := false
	seenVersions := false
	seenFiles := 0
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			t.Fatalf("read tar: %v", err)
		}
		switch hdr.Name {
		case "docs.json":
			seenDocs = true
			var got []map[string]interface{}
			if err := json.NewDecoder(tr).Decode(&got); err != nil {
				t.Fatalf("decode docs.json: %v", err)
			}
			if len(got) != len(docs) {
				t.Fatalf("expected %d docs, got %d", len(docs), len(got))
			}
		case "meta.json":
			seenMeta = true
			var meta map[string]interface{}
			if err := json.NewDecoder(tr).Decode(&meta); err != nil {
				t.Fatalf("decode meta.json: %v", err)
			}
			if meta["doc_count"].(float64) != float64(len(docs)) {
				t.Fatalf("meta doc_count mismatch")
			}
		case "versions.json":
			seenVersions = true
			var payload []map[string]interface{}
			if err := json.NewDecoder(tr).Decode(&payload); err != nil {
				t.Fatalf("decode versions.json: %v", err)
			}
			if len(payload) != 1 {
				t.Fatalf("expected versions for 1 doc")
			}
		default:
			if strings.HasPrefix(hdr.Name, "docs/") {
				seenFiles++
				data, err := io.ReadAll(tr)
				if err != nil {
					t.Fatalf("read doc file: %v", err)
				}
				if len(data) == 0 {
					t.Fatalf("doc file %s empty", hdr.Name)
				}
				continue
			}
			t.Fatalf("unexpected tar entry %s", hdr.Name)
		}
	}
	if !seenDocs || !seenMeta || !seenVersions || seenFiles == 0 {
		t.Fatalf("expected docs.json, versions.json, meta.json, and docs/*.md entries")
	}
}
