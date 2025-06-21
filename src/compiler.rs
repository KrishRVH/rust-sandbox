// A robust compiler for the "Tiny" language with stochastic variables
//
// Language Specification:
// =======================
// Keywords:
//   - "blah" : prints a value (string or numeric expression)
//   - "maybe" : declares a variable with 50% chance of being null
//
// Syntax:
//   program     := statement*
//   statement   := maybe_decl | blah_stmt
//   maybe_decl  := "maybe" IDENTIFIER "=" expression
//   blah_stmt   := "blah" expression
//   expression  := term (("+"|"-") term)*
//   term        := NUMBER | STRING | IDENTIFIER
//The Language Rules:
//Variables must be declared before use
//Variables can only be declared once
//Strings can only be printed, not used in arithmetic
//Variables can only hold numbers (which might be null)
//Arithmetic operations treat null as 0
//Comments start with // and continue to end of line
// Semantics:
//   - Variables declared with "maybe" have a 50% chance of being assigned
//     their value, and a 50% chance of being null
//   - Null variables evaluate to 0 in arithmetic expressions
//   - Undefined variables are an error
//   - String literals can only appear in blah statements, not in arithmetic

use std::fs;
use std::io::Write;
use std::process::Command;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Position in source code for better error reporting
#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    line: usize,
    column: usize,
}

impl Position {
    fn new() -> Self {
        Position { line: 1, column: 1 }
    }

    fn advance(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Compilation error with position information
#[derive(Debug, Clone)]
struct CompileError {
    message: String,
    position: Position,
}

impl CompileError {
    fn new(message: String, position: Position) -> Self {
        CompileError { message, position }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error at {}: {}", self.position, self.message)
    }
}

type Result<T> = std::result::Result<T, CompileError>;

/// Token types in our tiny language
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Blah,                      // The "blah" keyword (print)
    Maybe,                     // The "maybe" keyword (variable declaration)
    Identifier(String),        // Variable names
    Number(i32),              // Integer literals
    StringLiteral(String),     // String literals
    Plus,                      // + operator
    Minus,                     // - operator
    Equals,                    // = operator
    Eof,                       // End of file
}

/// Token with position information
#[derive(Debug, Clone)]
struct TokenWithPos {
    token: Token,
    position: Position,
}

/// Lexer: Converts source code into tokens
/// 
/// The lexer performs lexical analysis, breaking the source code into
/// a sequence of tokens. It handles:
/// - Keywords (blah, maybe)
/// - Identifiers (variable names)
/// - Numbers (positive integers only)
/// - String literals (enclosed in double quotes)
/// - Operators (+, -, =)
/// - Whitespace (which is ignored)
struct Lexer {
    input: Vec<char>,
    position: usize,
    current_pos: Position,
}

impl Lexer {
    /// Create a new lexer from source code
    fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            current_pos: Position::new(),
        }
    }

    /// Peek at the current character without consuming it
    fn peek(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    /// Peek ahead n characters
    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.input.get(self.position + n).copied()
    }

    /// Consume and return the current character
    fn advance(&mut self) -> Option<char> {
        let ch = self.peek();
        if let Some(c) = ch {
            self.position += 1;
            self.current_pos.advance(c);
        }
        ch
    }

    /// Skip whitespace and comments
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '/' && self.peek_ahead(1) == Some('/') {
                // Skip line comments
                self.advance(); // Skip first /
                self.advance(); // Skip second /
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    /// Read a word (identifier or keyword)
    /// Valid identifiers start with a letter or underscore,
    /// followed by letters, digits, or underscores
    fn read_word(&mut self) -> String {
        let mut word = String::new();
        
        // First character must be letter or underscore
        if let Some(ch) = self.peek() {
            if ch.is_alphabetic() || ch == '_' {
                word.push(ch);
                self.advance();
            }
        }
        
        // Subsequent characters can be letters, digits, or underscores
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                word.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        word
    }

    /// Read a number (positive integers only)
    fn read_number(&mut self) -> Result<i32> {
        let start_pos = self.current_pos;
        let mut num = String::new();
        
        while let Some(ch) = self.peek() {
            if ch.is_numeric() {
                num.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        num.parse()
            .map_err(|_| CompileError::new(
                format!("Invalid number: {}", num),
                start_pos
            ))
    }

    /// Read a string literal (text between double quotes)
    /// Supports basic escape sequences: \n, \t, \\, \"
    fn read_string_literal(&mut self) -> Result<String> {
        let start_pos = self.current_pos;
        
        // Skip the opening quote
        self.advance();
        
        let mut string = String::new();
        let mut escaped = false;
        
        loop {
            match self.advance() {
                Some('\\') if !escaped => {
                    escaped = true;
                }
                Some('n') if escaped => {
                    string.push('\n');
                    escaped = false;
                }
                Some('t') if escaped => {
                    string.push('\t');
                    escaped = false;
                }
                Some('\\') if escaped => {
                    string.push('\\');
                    escaped = false;
                }
                Some('"') if escaped => {
                    string.push('"');
                    escaped = false;
                }
                Some('"') if !escaped => {
                    return Ok(string);
                }
                Some(ch) => {
                    if escaped {
                        return Err(CompileError::new(
                            format!("Invalid escape sequence: \\{}", ch),
                            self.current_pos
                        ));
                    }
                    string.push(ch);
                }
                None => {
                    return Err(CompileError::new(
                        "Unterminated string literal".to_string(),
                        start_pos
                    ));
                }
            }
        }
    }

    /// Get the next token from the input
    fn next_token(&mut self) -> Result<TokenWithPos> {
        self.skip_whitespace();
        
        let start_pos = self.current_pos;
        
        match self.peek() {
            None => Ok(TokenWithPos {
                token: Token::Eof,
                position: start_pos,
            }),
            Some('"') => {
                let string = self.read_string_literal()?;
                Ok(TokenWithPos {
                    token: Token::StringLiteral(string),
                    position: start_pos,
                })
            }
            Some('+') => {
                self.advance();
                Ok(TokenWithPos {
                    token: Token::Plus,
                    position: start_pos,
                })
            }
            Some('-') => {
                self.advance();
                Ok(TokenWithPos {
                    token: Token::Minus,
                    position: start_pos,
                })
            }
            Some('=') => {
                self.advance();
                Ok(TokenWithPos {
                    token: Token::Equals,
                    position: start_pos,
                })
            }
            Some(ch) if ch.is_numeric() => {
                let num = self.read_number()?;
                Ok(TokenWithPos {
                    token: Token::Number(num),
                    position: start_pos,
                })
            }
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                let word = self.read_word();
                let token = match word.as_str() {
                    "blah" => Token::Blah,
                    "maybe" => Token::Maybe,
                    _ => Token::Identifier(word),
                };
                Ok(TokenWithPos {
                    token,
                    position: start_pos,
                })
            }
            Some(ch) => Err(CompileError::new(
                format!("Unexpected character: '{}'", ch),
                start_pos
            )),
        }
    }

    /// Tokenize the entire input
    fn tokenize(&mut self) -> Result<Vec<TokenWithPos>> {
        let mut tokens = Vec::new();
        
        loop {
            let token_with_pos = self.next_token()?;
            let is_eof = matches!(token_with_pos.token, Token::Eof);
            tokens.push(token_with_pos);
            
            if is_eof {
                break;
            }
        }
        
        Ok(tokens)
    }
}

/// Expression types for arithmetic and values
#[derive(Debug, Clone, PartialEq)]
enum Expression {
    Number(i32),
    Variable(String),
    StringLiteral(String),
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
}

/// AST (Abstract Syntax Tree) node types
#[derive(Debug, Clone, PartialEq)]
enum AstNode {
    BlahStatement(Expression),           // Blah (print) statement
    MaybeDeclaration(String, Expression), // Variable declaration with 50% chance
}

/// Parser: Converts tokens into an AST
/// 
/// The parser performs syntax analysis, building an Abstract Syntax Tree
/// from the token stream. It enforces the grammar rules and reports
/// syntax errors with position information.
struct Parser {
    tokens: Vec<TokenWithPos>,
    position: usize,
    declared_vars: HashSet<String>,
}

impl Parser {
    /// Create a new parser from tokens
    fn new(tokens: Vec<TokenWithPos>) -> Self {
        Parser {
            tokens,
            position: 0,
            declared_vars: HashSet::new(),
        }
    }

    /// Peek at the current token without consuming it
    fn peek(&self) -> &Token {
        &self.tokens
            .get(self.position)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    /// Get the position of the current token
    fn current_position(&self) -> Position {
        self.tokens
            .get(self.position)
            .map(|t| t.position)
            .unwrap_or(Position::new())
    }

    /// Consume and return the current token with position
    fn advance(&mut self) -> TokenWithPos {
        let token = self.tokens
            .get(self.position)
            .cloned()
            .unwrap_or(TokenWithPos {
                token: Token::Eof,
                position: Position::new(),
            });
        self.position += 1;
        token
    }

    /// Parse a primary expression (number, variable, or string)
    fn parse_primary(&mut self) -> Result<Expression> {
        let token_with_pos = self.advance();
        let pos = token_with_pos.position;
        
        match token_with_pos.token {
            Token::Number(n) => Ok(Expression::Number(n)),
            Token::Identifier(name) => {
                // Check if variable has been declared
                if !self.declared_vars.contains(&name) {
                    return Err(CompileError::new(
                        format!("Undefined variable: '{}'", name),
                        pos
                    ));
                }
                Ok(Expression::Variable(name))
            }
            Token::StringLiteral(s) => Ok(Expression::StringLiteral(s)),
            _ => Err(CompileError::new(
                "Expected number, variable, or string".to_string(),
                pos
            )),
        }
    }

    /// Parse an expression with addition and subtraction
    /// Uses left-to-right associativity
    fn parse_expression(&mut self) -> Result<Expression> {
        let mut left = self.parse_primary()?;

        loop {
            let pos = self.current_position();
            match self.peek() {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_primary()?;
                    
                    // Type checking: can't add strings
                    if matches!(&left, Expression::StringLiteral(_)) || 
                       matches!(&right, Expression::StringLiteral(_)) {
                        return Err(CompileError::new(
                            "Cannot use strings in arithmetic expressions".to_string(),
                            pos
                        ));
                    }
                    
                    left = Expression::Add(Box::new(left), Box::new(right));
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_primary()?;
                    
                    // Type checking: can't subtract strings
                    if matches!(&left, Expression::StringLiteral(_)) || 
                       matches!(&right, Expression::StringLiteral(_)) {
                        return Err(CompileError::new(
                            "Cannot use strings in arithmetic expressions".to_string(),
                            pos
                        ));
                    }
                    
                    left = Expression::Subtract(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parse a blah statement: blah expression
    fn parse_blah_statement(&mut self) -> Result<AstNode> {
        let pos = self.current_position();
        
        // Expect "blah" keyword
        match self.advance().token {
            Token::Blah => {}
            _ => return Err(CompileError::new(
                "Expected 'blah' keyword".to_string(),
                pos
            )),
        }

        // Parse the expression to print
        let expr = self.parse_expression()?;
        Ok(AstNode::BlahStatement(expr))
    }

    /// Parse a maybe declaration: maybe varname = expression
    fn parse_maybe_declaration(&mut self) -> Result<AstNode> {
        let pos = self.current_position();
        
        // Expect "maybe" keyword
        match self.advance().token {
            Token::Maybe => {}
            _ => return Err(CompileError::new(
                "Expected 'maybe' keyword".to_string(),
                pos
            )),
        }

        let var_pos = self.current_position();
        
        // Expect variable name
        let var_name = match self.advance().token {
            Token::Identifier(name) => {
                // Check for redeclaration
                if self.declared_vars.contains(&name) {
                    return Err(CompileError::new(
                        format!("Variable '{}' already declared", name),
                        var_pos
                    ));
                }
                name
            }
            _ => return Err(CompileError::new(
                "Expected variable name after 'maybe'".to_string(),
                var_pos
            )),
        };

        // Expect equals sign
        let eq_pos = self.current_position();
        match self.advance().token {
            Token::Equals => {}
            _ => return Err(CompileError::new(
                "Expected '=' after variable name".to_string(),
                eq_pos
            )),
        }

        // Parse the expression
        let expr = self.parse_expression()?;
        
        // Strings can't be assigned to variables
        if matches!(&expr, Expression::StringLiteral(_)) {
            return Err(CompileError::new(
                "Cannot assign string literals to variables".to_string(),
                self.current_position()
            ));
        }

        // Mark variable as declared
        self.declared_vars.insert(var_name.clone());

        Ok(AstNode::MaybeDeclaration(var_name, expr))
    }

    /// Parse a statement (either blah or maybe)
    fn parse_statement(&mut self) -> Result<AstNode> {
        let pos = self.current_position();
        
        match self.peek() {
            Token::Blah => self.parse_blah_statement(),
            Token::Maybe => self.parse_maybe_declaration(),
            Token::Eof => Err(CompileError::new(
                "Unexpected end of file".to_string(),
                pos
            )),
            _ => Err(CompileError::new(
                "Expected 'blah' or 'maybe' statement".to_string(),
                pos
            )),
        }
    }

    /// Parse the entire program
    fn parse(&mut self) -> Result<Vec<AstNode>> {
        let mut statements = Vec::new();

        while self.peek() != &Token::Eof {
            let statement = self.parse_statement()?;
            statements.push(statement);
        }

        Ok(statements)
    }
}

/// Code Generator: Converts AST to C code
/// 
/// The code generator performs the final phase of compilation,
/// transforming the AST into executable C code. It handles:
/// - Variable declarations with null tracking
/// - Expression evaluation with null safety
/// - Random number generation for the "maybe" feature
struct CodeGenerator;

impl CodeGenerator {
    /// Generate C code for an expression
    fn generate_expression(&self, expr: &Expression) -> String {
        match expr {
            Expression::Number(n) => n.to_string(),
            Expression::Variable(name) => {
                // Variables might be null, so we need to check
                // If null, evaluate to 0
                format!("({}_is_null ? 0 : {})", name, name)
            }
            Expression::StringLiteral(s) => {
                // Escape special characters for C
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t");
                format!("\"{}\"", escaped)
            }
            Expression::Add(left, right) => {
                format!("({} + {})", 
                    self.generate_expression(left), 
                    self.generate_expression(right))
            }
            Expression::Subtract(left, right) => {
                format!("({} - {})", 
                    self.generate_expression(left), 
                    self.generate_expression(right))
            }
        }
    }

    /// Generate C code from an AST
    fn generate(&self, ast: Vec<AstNode>) -> String {
        let mut code = String::new();
        
        // C headers
        code.push_str("// Generated by Tiny Language Compiler\n");
        code.push_str("#include <stdio.h>\n");
        code.push_str("#include <stdlib.h>\n");
        code.push_str("#include <time.h>\n\n");
        
        // Main function
        code.push_str("int main() {\n");
        
        // Initialize random seed for the "maybe" feature
        code.push_str("    // Initialize random number generator\n");
        code.push_str("    srand(time(NULL));\n\n");
        
        // Generate code for each statement
        for node in ast {
            match node {
                AstNode::BlahStatement(expr) => {
                    match &expr {
                        Expression::StringLiteral(_) => {
                            // String literals are printed with %s format
                            code.push_str(&format!("    printf(\"%s\\n\", {});\n", 
                                self.generate_expression(&expr)));
                        }
                        _ => {
                            // Numeric expressions are printed with %d format
                            code.push_str(&format!("    printf(\"%d\\n\", {});\n", 
                                self.generate_expression(&expr)));
                        }
                    }
                }
                AstNode::MaybeDeclaration(var_name, expr) => {
                    // Generate variable declaration with null tracking
                    code.push_str(&format!("    // Declare variable '{}' with 50% chance of null\n", var_name));
                    code.push_str(&format!("    int {} = 0;\n", var_name));
                    code.push_str(&format!("    int {}_is_null = 0;\n", var_name));
                    
                    // 50% chance of being null
                    code.push_str("    if (rand() % 2 == 0) {\n");
                    code.push_str(&format!("        {} = {};\n", var_name, 
                        self.generate_expression(&expr)));
                    code.push_str(&format!("        printf(\"maybe {} = %d\\n\", {});\n", 
                        var_name, var_name));
                    code.push_str("    } else {\n");
                    code.push_str(&format!("        {}_is_null = 1;\n", var_name));
                    code.push_str(&format!("        printf(\"maybe {} = null\\n\");\n", var_name));
                    code.push_str("    }\n\n");
                }
            }
        }
        
        code.push_str("    return 0;\n");
        code.push_str("}\n");
        
        code
    }
}

/// The main compiler function
fn compile(source: &str) -> Result<String> {
    // Step 1: Lexical analysis (tokenization)
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    
    println!("=== TOKENS ===");
    for token in &tokens {
        println!("{:?} at {}", token.token, token.position);
    }
    println!();
    
    // Step 2: Parsing (build AST)
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    println!("=== AST ===");
    for (i, node) in ast.iter().enumerate() {
        println!("{}: {:?}", i, node);
    }
    println!();
    
    // Step 3: Code generation
    let generator = CodeGenerator;
    let c_code = generator.generate(ast);
    
    println!("=== GENERATED C CODE ===");
    println!("{}", c_code);
    
    Ok(c_code)
}

/// Example usage and testing
fn main() {
    // Example Tiny program with stochastic variables
    let source = r#"// Tiny program demonstrating stochastic variables
maybe x = 10
maybe y = 5
maybe z = 3

blah "Calculating with maybe variables..."
blah x + y
blah x - z
blah x + y - z"#;

    println!("=== TINY SOURCE CODE ===");
    println!("{}", source);
    println!();

    // Compile the Tiny program
    match compile(source) {
        Ok(c_code) => {
            // Write C code to file
            let mut file = fs::File::create("output.c").expect("Failed to create file");
            file.write_all(c_code.as_bytes()).expect("Failed to write file");
            
            println!("\nC code written to output.c");
            
            // Optionally compile and run the C code
            println!("\nCompiling C code...");
            let output = Command::new("gcc")
                .args(&["-o", "output", "output.c", "-Wall", "-Wextra"])
                .output();
                
            match output {
                Ok(compilation) => {
                    if compilation.status.success() {
                        println!("Compilation successful! Running program:\n");
                        println!("Note: Variables have 50% chance of being null!\n");
                        
                        // Run the program multiple times to see the stochastic behavior
                        for i in 1..=3 {
                            println!("--- Run {} ---", i);
                            let run_output = Command::new("./output").output();
                            if let Ok(result) = run_output {
                                print!("{}", String::from_utf8_lossy(&result.stdout));
                            }
                            println!();
                        }
                    } else {
                        eprintln!("C compilation failed:");
                        eprintln!("{}", String::from_utf8_lossy(&compilation.stderr));
                    }
                }
                Err(_) => {
                    println!("GCC not found. You can compile output.c manually with:");
                    println!("  gcc -o output output.c");
                }
            }
        }
        Err(e) => {
            eprintln!("Compilation error: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_keywords() {
        let mut lexer = Lexer::new("blah maybe");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Blah);
        assert_eq!(tokens[1].token, Token::Maybe);
        assert_eq!(tokens[2].token, Token::Eof);
    }

    #[test]
    fn test_lexer_identifiers() {
        let mut lexer = Lexer::new("x y123 _test test_var");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Identifier("x".to_string()));
        assert_eq!(tokens[1].token, Token::Identifier("y123".to_string()));
        assert_eq!(tokens[2].token, Token::Identifier("_test".to_string()));
        assert_eq!(tokens[3].token, Token::Identifier("test_var".to_string()));
    }

    #[test]
    fn test_lexer_numbers() {
        let mut lexer = Lexer::new("42 0 999");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Number(42));
        assert_eq!(tokens[1].token, Token::Number(0));
        assert_eq!(tokens[2].token, Token::Number(999));
    }

    #[test]
    fn test_lexer_strings() {
        let mut lexer = Lexer::new(r#""hello" "world" "with \"quotes\"""#);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::StringLiteral("hello".to_string()));
        assert_eq!(tokens[1].token, Token::StringLiteral("world".to_string()));
        assert_eq!(tokens[2].token, Token::StringLiteral("with \"quotes\"".to_string()));
    }

    #[test]
    fn test_lexer_operators() {
        let mut lexer = Lexer::new("+ - =");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Plus);
        assert_eq!(tokens[1].token, Token::Minus);
        assert_eq!(tokens[2].token, Token::Equals);
    }

    #[test]
    fn test_lexer_comments() {
        let mut lexer = Lexer::new("blah // this is a comment\nmaybe");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].token, Token::Blah);
        assert_eq!(tokens[1].token, Token::Maybe);
        assert_eq!(tokens[2].token, Token::Eof);
    }

    #[test]
    fn test_lexer_position_tracking() {
        let mut lexer = Lexer::new("blah\nmaybe");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].position, Position { line: 1, column: 1 });
        assert_eq!(tokens[1].position, Position { line: 2, column: 1 });
    }

    #[test]
    fn test_lexer_error_invalid_char() {
        let mut lexer = Lexer::new("blah @");
        let result = lexer.tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unexpected character"));
    }

    #[test]
    fn test_lexer_error_unterminated_string() {
        let mut lexer = Lexer::new(r#""unterminated"#);
        let result = lexer.tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unterminated string"));
    }

    #[test]
    fn test_parser_blah_number() {
        let tokens = vec![
            TokenWithPos { token: Token::Blah, position: Position::new() },
            TokenWithPos { token: Token::Number(42), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0], AstNode::BlahStatement(Expression::Number(42)));
    }

    #[test]
    fn test_parser_blah_string() {
        let tokens = vec![
            TokenWithPos { token: Token::Blah, position: Position::new() },
            TokenWithPos { token: Token::StringLiteral("hello".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0], AstNode::BlahStatement(Expression::StringLiteral("hello".to_string())));
    }

    #[test]
    fn test_parser_maybe_declaration() {
        let tokens = vec![
            TokenWithPos { token: Token::Maybe, position: Position::new() },
            TokenWithPos { token: Token::Identifier("x".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Equals, position: Position::new() },
            TokenWithPos { token: Token::Number(10), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0], AstNode::MaybeDeclaration("x".to_string(), Expression::Number(10)));
    }

    #[test]
    fn test_parser_arithmetic_add() {
        let tokens = vec![
            TokenWithPos { token: Token::Blah, position: Position::new() },
            TokenWithPos { token: Token::Number(5), position: Position::new() },
            TokenWithPos { token: Token::Plus, position: Position::new() },
            TokenWithPos { token: Token::Number(3), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0], AstNode::BlahStatement(
            Expression::Add(
                Box::new(Expression::Number(5)),
                Box::new(Expression::Number(3))
            )
        ));
    }

    #[test]
    fn test_parser_arithmetic_chain() {
        let tokens = vec![
            TokenWithPos { token: Token::Blah, position: Position::new() },
            TokenWithPos { token: Token::Number(10), position: Position::new() },
            TokenWithPos { token: Token::Plus, position: Position::new() },
            TokenWithPos { token: Token::Number(5), position: Position::new() },
            TokenWithPos { token: Token::Minus, position: Position::new() },
            TokenWithPos { token: Token::Number(3), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.len(), 1);
        // Should parse as (10 + 5) - 3 due to left-to-right associativity
        assert_eq!(ast[0], AstNode::BlahStatement(
            Expression::Subtract(
                Box::new(Expression::Add(
                    Box::new(Expression::Number(10)),
                    Box::new(Expression::Number(5))
                )),
                Box::new(Expression::Number(3))
            )
        ));
    }

    #[test]
    fn test_parser_variable_usage() {
        let tokens = vec![
            TokenWithPos { token: Token::Maybe, position: Position::new() },
            TokenWithPos { token: Token::Identifier("x".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Equals, position: Position::new() },
            TokenWithPos { token: Token::Number(10), position: Position::new() },
            TokenWithPos { token: Token::Blah, position: Position::new() },
            TokenWithPos { token: Token::Identifier("x".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.len(), 2);
        assert_eq!(ast[0], AstNode::MaybeDeclaration("x".to_string(), Expression::Number(10)));
        assert_eq!(ast[1], AstNode::BlahStatement(Expression::Variable("x".to_string())));
    }

    #[test]
    fn test_parser_error_undefined_variable() {
        let tokens = vec![
            TokenWithPos { token: Token::Blah, position: Position::new() },
            TokenWithPos { token: Token::Identifier("undefined".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Undefined variable"));
    }

    #[test]
    fn test_parser_error_redeclaration() {
        let tokens = vec![
            TokenWithPos { token: Token::Maybe, position: Position::new() },
            TokenWithPos { token: Token::Identifier("x".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Equals, position: Position::new() },
            TokenWithPos { token: Token::Number(10), position: Position::new() },
            TokenWithPos { token: Token::Maybe, position: Position::new() },
            TokenWithPos { token: Token::Identifier("x".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Equals, position: Position::new() },
            TokenWithPos { token: Token::Number(20), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("already declared"));
    }

    #[test]
    fn test_parser_error_string_in_arithmetic() {
        let tokens = vec![
            TokenWithPos { token: Token::Blah, position: Position::new() },
            TokenWithPos { token: Token::StringLiteral("hello".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Plus, position: Position::new() },
            TokenWithPos { token: Token::Number(5), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Cannot use strings in arithmetic"));
    }

    #[test]
    fn test_parser_error_string_assignment() {
        let tokens = vec![
            TokenWithPos { token: Token::Maybe, position: Position::new() },
            TokenWithPos { token: Token::Identifier("x".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Equals, position: Position::new() },
            TokenWithPos { token: Token::StringLiteral("hello".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Cannot assign string literals to variables"));
    }

    #[test]
    fn test_parser_error_missing_equals() {
        let tokens = vec![
            TokenWithPos { token: Token::Maybe, position: Position::new() },
            TokenWithPos { token: Token::Identifier("x".to_string()), position: Position::new() },
            TokenWithPos { token: Token::Number(10), position: Position::new() },
            TokenWithPos { token: Token::Eof, position: Position::new() },
        ];
        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected '='"));
    }

    #[test]
    fn test_code_generation_blah_number() {
        let ast = vec![AstNode::BlahStatement(Expression::Number(42))];
        let generator = CodeGenerator;
        let code = generator.generate(ast);
        assert!(code.contains("printf(\"%d\\n\", 42);"));
    }

    #[test]
    fn test_code_generation_blah_string() {
        let ast = vec![AstNode::BlahStatement(Expression::StringLiteral("hello".to_string()))];
        let generator = CodeGenerator;
        let code = generator.generate(ast);
        assert!(code.contains("printf(\"%s\\n\", \"hello\");"));
    }

    #[test]
    fn test_code_generation_maybe() {
        let ast = vec![AstNode::MaybeDeclaration("x".to_string(), Expression::Number(10))];
        let generator = CodeGenerator;
        let code = generator.generate(ast);
        assert!(code.contains("int x = 0;"));
        assert!(code.contains("int x_is_null = 0;"));
        assert!(code.contains("rand() % 2 == 0"));
        assert!(code.contains("x = 10;"));
        assert!(code.contains("x_is_null = 1;"));
    }

    #[test]
    fn test_code_generation_variable_null_check() {
        let ast = vec![AstNode::BlahStatement(Expression::Variable("x".to_string()))];
        let generator = CodeGenerator;
        let code = generator.generate(ast);
        assert!(code.contains("(x_is_null ? 0 : x)"));
    }

    #[test]
    fn test_code_generation_arithmetic() {
        let ast = vec![AstNode::BlahStatement(
            Expression::Add(
                Box::new(Expression::Number(5)),
                Box::new(Expression::Number(3))
            )
        )];
        let generator = CodeGenerator;
        let code = generator.generate(ast);
        assert!(code.contains("(5 + 3)"));
    }

    #[test]
    fn test_code_generation_escape_sequences() {
        let ast = vec![AstNode::BlahStatement(
            Expression::StringLiteral("hello\nworld\t\"quoted\"".to_string())
        )];
        let generator = CodeGenerator;
        let code = generator.generate(ast);
        assert!(code.contains("hello\\nworld\\t\\\"quoted\\\""));
    }

    #[test]
    fn test_integration_simple_program() {
        let source = r#"
            maybe x = 10
            blah x
        "#;
        let result = compile(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("int x = 0;"));
        assert!(code.contains("int x_is_null = 0;"));
        assert!(code.contains("printf(\"%d\\n\", (x_is_null ? 0 : x));"));
    }

    #[test]
    fn test_integration_arithmetic_program() {
        let source = r#"
            maybe x = 10
            maybe y = 5
            blah x + y
            blah x - y
        "#;
        let result = compile(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("((x_is_null ? 0 : x) + (y_is_null ? 0 : y))"));
        assert!(code.contains("((x_is_null ? 0 : x) - (y_is_null ? 0 : y))"));
    }

    #[test]
    fn test_integration_error_handling() {
        // Test undefined variable
        let source = "blah undefined_var";
        let result = compile(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Undefined variable"));

        // Test invalid syntax
        let source = "blah +";
        let result = compile(source);
        assert!(result.is_err());

        // Test string in arithmetic
        let source = r#"blah "hello" + 5"#;
        let result = compile(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Cannot use strings in arithmetic"));
    }

    #[test]
    fn test_empty_program() {
        let source = "";
        let result = compile(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("int main()"));
        assert!(code.contains("return 0;"));
    }

    #[test]
    fn test_whitespace_handling() {
        let source = "   maybe   x   =   10   \n   blah   x   ";
        let result = compile(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_comment_handling() {
        let source = r#"
            // This is a comment
            maybe x = 10  // Another comment
            blah x
            // Final comment
        "#;
        let result = compile(source);
        assert!(result.is_ok());
    }
}