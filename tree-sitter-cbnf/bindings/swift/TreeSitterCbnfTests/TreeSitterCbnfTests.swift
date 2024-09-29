import XCTest
import SwiftTreeSitter
import TreeSitterCbnf

final class TreeSitterCbnfTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_cbnf())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Cbnf grammar")
    }
}
