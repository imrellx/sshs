# Claude Software Engineering Assistant v3.5

<context>
Core development standards and practices. This file should exist in every project root.
</context>

## Quick Reference
- **Ask Permission**: Before major refactoring or architectural changes
- **TDD Default**: Tests first, except when refactoring well-tested code  
- **Production Ready**: Include error handling, logging, monitoring
- **Security First**: Never hardcode secrets, validate inputs
- **Match Style**: Follow existing patterns in the codebase

---

## Core Standards

### Code Quality
- Readable > Clever
- Small, focused functions
- Meaningful names
- Document WHY, not WHAT

### Testing (TDD)
1. Red: Write failing test
2. Green: Minimal code to pass
3. Refactor: Improve while green
4. Repeat

**Test Quality**: Assert behavior, not implementation. Cover edge cases.

### Git Commits
- Format: `type: description`
- Types: feat, fix, docs, style, refactor, test, chore
- Keep under 50 characters
- Reference issues: "Closes #123"

### Security Basics
- Environment variables for secrets
- Validate all inputs
- Use parameterized queries
- Hash passwords properly (bcrypt/argon2)
- HTTPS everywhere

### Error Handling
- Catch specific exceptions
- Log errors with context
- User-friendly error messages
- Always clean up resources

---

## Communication & Permissions

### Permission Protocol
Before making substantial changes, ask clearly:
> "The existing implementation of X has issues (A, B, C). I could address these by refactoring it, which would involve Y. This approach offers Z benefits. Would you like me to proceed?"

### When to Ask Permission
- Changing core architecture
- Refactoring >100 lines
- Adding new dependencies
- Changing public APIs
- Modifying critical business logic

### Response Preferences
- **Code First**: Lead with working solutions, explain after
- **Complete Solutions**: No skeletons or placeholders
- **Clear Structure**: Use sections like `## Implementation`, `## Testing Notes`

---

## Claude 4 Optimizations

### Enhanced Capabilities
- **Parallel Operations**: Use multiple tools simultaneously when analyzing code or files
- **Explicit Instructions**: Be specific - "Include retry logic with exponential backoff"
- **Quality Modifiers**: Add "production-ready", "with comprehensive error handling"
- **Structured Output**: Request specific formats when needed

### Effective Prompting
- Provide context about WHY, not just WHAT
- Include examples of desired patterns
- Specify edge cases explicitly
- Request tests alongside implementation

---

## Context-Specific Guidelines

### When Performance Matters
*Apply only when performance is a stated requirement:*
- Measure before optimizing
- Document performance-critical sections
- Consider algorithmic complexity first
- Avoid premature optimization

### When Refactoring
*Apply when modifying well-tested existing code:*
- Ensure test coverage first
- Make incremental changes
- Keep tests green throughout
- Document why refactoring is needed

### When Debugging
*Apply when investigating issues:*
- Reproduce consistently first
- Add strategic logging
- Explain root cause, not just fix
- Document non-obvious findings

---

## Python Specific
- Use `uv` for package management
- Type hints for public APIs
- Follow PEP 8 with project variations
- Docstrings for public functions

## Project Structure
```
project/
├── claude.md        # This file
├── README.md        # Project overview
├── pyproject.toml   # Dependencies
├── todo.md          # Task tracking
├── src/             # Source code
└── tests/           # Test files
```

---

## Remember
- Ship working code frequently
- Perfect is the enemy of good
- Tests are documentation
- Code for the next developer
- When in doubt, ask