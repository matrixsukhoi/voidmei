package prog.config;

import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * A zero-dependency S-Expression parser.
 * Supports:
 * - Lists: (a b c)
 * - Strings: "hello world"
 * - Numbers: 123, 12.34
 * - Booleans: true, false
 * - Keywords: :x, :type
 * - Symbols: panel, group
 */
public class SExpParser {

    // --- AST Nodes ---

    public interface SExp {
        boolean isList();

        boolean isAtom();

        SList asList();

        SAtom asAtom();
    }

    public static class SList implements SExp {
        public List<SExp> children = new ArrayList<>();

        public void add(SExp exp) {
            children.add(exp);
        }

        @Override
        public boolean isList() {
            return true;
        }

        @Override
        public boolean isAtom() {
            return false;
        }

        @Override
        public SList asList() {
            return this;
        }

        @Override
        public SAtom asAtom() {
            throw new IllegalStateException("Not an atom");
        }

        @Override
        public String toString() {
            StringBuilder sb = new StringBuilder("(");
            for (int i = 0; i < children.size(); i++) {
                sb.append(children.get(i).toString());
                if (i < children.size() - 1)
                    sb.append(" ");
            }
            sb.append(")");
            return sb.toString();
        }
    }

    public static class SAtom implements SExp {
        public String value;
        public AtomType type;

        public enum AtomType {
            STRING, NUMBER, BOOLEAN, KEYWORD, SYMBOL
        }

        public SAtom(String value, AtomType type) {
            this.value = value;
            this.type = type;
        }

        public String getString() {
            return value;
        }

        public double getDouble() {
            return Double.parseDouble(value);
        }

        public int getInt() {
            return (int) getDouble();
        }

        public boolean getBool() {
            return Boolean.parseBoolean(value);
        }

        public boolean isKeyword() {
            return type == AtomType.KEYWORD;
        }

        public boolean isSymbol() {
            return type == AtomType.SYMBOL;
        }

        @Override
        public boolean isList() {
            return false;
        }

        @Override
        public boolean isAtom() {
            return true;
        }

        @Override
        public SList asList() {
            throw new IllegalStateException("Not a list");
        }

        @Override
        public SAtom asAtom() {
            return this;
        }

        @Override
        public String toString() {
            if (type == AtomType.STRING)
                return "\"" + value + "\"";
            return value;
        }
    }

    // --- Tokenizer ---

    private enum TokenType {
        LPAREN, RPAREN, STRING, NUMBER, BOOLEAN, KEYWORD, SYMBOL, EOF
    }

    private static class Token {
        TokenType type;
        String value;

        Token(TokenType type, String value) {
            this.type = type;
            this.value = value;
        }
    }

    private List<Token> tokenize(String input) {
        List<Token> tokens = new ArrayList<>();
        // Simple regex-based tokenizer
        // 1. Strings: "..."
        // 2. Comments: ;... (handled by pre-processing or regex)
        // 3. Special chars: ( )
        // 4. Whitespace (ignored)
        // 5. Atoms: everything else

        int i = 0;
        int len = input.length();
        while (i < len) {
            char c = input.charAt(i);

            if (Character.isWhitespace(c)) {
                i++;
                continue;
            }

            if (c == ';') {
                // Comment, skip to end of line
                while (i < len && input.charAt(i) != '\n')
                    i++;
                continue;
            }

            if (c == '(') {
                tokens.add(new Token(TokenType.LPAREN, "("));
                i++;
                continue;
            }

            if (c == ')') {
                tokens.add(new Token(TokenType.RPAREN, ")"));
                i++;
                continue;
            }

            if (c == '"') {
                // String literal
                StringBuilder sb = new StringBuilder();
                i++; // skip open quote
                while (i < len) {
                    char sc = input.charAt(i);
                    if (sc == '"') {
                        i++; // skip close quote
                        break;
                    }
                    if (sc == '\\' && i + 1 < len) {
                        // Simple escape handling
                        i++;
                        sb.append(input.charAt(i));
                    } else {
                        sb.append(sc);
                    }
                    i++;
                }
                tokens.add(new Token(TokenType.STRING, sb.toString()));
                continue;
            }

            // Atom (Keyword, Number, Boolean, Symbol)
            StringBuilder sb = new StringBuilder();
            while (i < len) {
                char ac = input.charAt(i);
                if (Character.isWhitespace(ac) || ac == ')' || ac == '(' || ac == ';')
                    break;
                sb.append(ac);
                i++;
            }
            String atomStr = sb.toString();
            if (atomStr.isEmpty())
                continue;

            if (atomStr.startsWith(":")) {
                tokens.add(new Token(TokenType.KEYWORD, atomStr));
            } else if (atomStr.equals("true") || atomStr.equals("false")) {
                tokens.add(new Token(TokenType.BOOLEAN, atomStr));
            } else if (isNumber(atomStr)) {
                tokens.add(new Token(TokenType.NUMBER, atomStr));
            } else {
                tokens.add(new Token(TokenType.SYMBOL, atomStr));
            }
        }
        tokens.add(new Token(TokenType.EOF, ""));
        return tokens;
    }

    private boolean isNumber(String s) {
        try {
            Double.parseDouble(s);
            return true;
        } catch (NumberFormatException e) {
            return false;
        }
    }

    // --- Parser ---

    private int pos;
    private List<Token> tokens;

    public List<SExp> parse(String input) {
        this.tokens = tokenize(input);
        this.pos = 0;
        List<SExp> expressions = new ArrayList<>();

        while (peek().type != TokenType.EOF) {
            expressions.add(parseExpression());
        }
        return expressions;
    }

    private Token peek() {
        if (pos >= tokens.size())
            return new Token(TokenType.EOF, "");
        return tokens.get(pos);
    }

    private Token consume() {
        Token t = peek();
        pos++;
        return t;
    }

    private SExp parseExpression() {
        Token t = peek();
        if (t.type == TokenType.LPAREN) {
            return parseList();
        } else {
            return parseAtom();
        }
    }

    private SList parseList() {
        consume(); // (
        SList list = new SList();
        while (peek().type != TokenType.RPAREN && peek().type != TokenType.EOF) {
            list.add(parseExpression());
        }
        consume(); // )
        return list;
    }

    private SAtom parseAtom() {
        Token t = consume();
        SAtom.AtomType type = SAtom.AtomType.SYMBOL;
        switch (t.type) {
            case STRING:
                type = SAtom.AtomType.STRING;
                break;
            case NUMBER:
                type = SAtom.AtomType.NUMBER;
                break;
            case BOOLEAN:
                type = SAtom.AtomType.BOOLEAN;
                break;
            case KEYWORD:
                type = SAtom.AtomType.KEYWORD;
                break;
            default:
                type = SAtom.AtomType.SYMBOL;
                break;
        }
        return new SAtom(t.value, type);
    }
}
