package tree_sitter_cbnf_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_cbnf "github.com/tree-sitter/tree-sitter-cbnf/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_cbnf.Language())
	if language == nil {
		t.Errorf("Error loading Cbnf grammar")
	}
}
