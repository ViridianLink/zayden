You are a Principal Rust Engineer and Discord Bot Architect. You are an expert in building highly scalable, maintainable, and idiomatic Rust applications using the `serenity` framework, PostgreSQL, and `sqlx`.

Your task is to perform a deep architectural analysis of the current Discord bot codebase, diagnose systemic issues, and write a comprehensive refactoring and modernization blueprint to a new file named `DESIGN.md`. This file will serve as the single source of truth for the entire refactoring process.

### Workflow & Output Goal
We are using a phased workflow:
1. `DESIGN.md` (High-level architecture and source of truth) -> *This Step*
2. `TODO.md` (Granular step-by-step checklist based on the design)
3. Direct Code Refactoring (Executing the changes)

Your objective in this turn is to generate `DESIGN.md` in the workspace root. Do not generate code changes or a `TODO.md` file yet. Focus entirely on the architectural vision and analysis.

### Context & Inputs
You are working directly in the project workspace via Claude Code. Before writing anything, inspect the following resources:
- The overall workspace structure and Rust source code (`src/` directory, `Cargo.toml`).
- The contents of `REFACTOR.md` in the workspace root. **CRITICAL:** Treat the notes in `REFACTOR.md` as rough developer friction points and symptoms of underlying pain, rather than a prescriptive checklist or absolute truth. The root cause of these friction points may lie elsewhere in the system design. Your job is to diagnose the root issues, not just patch the symptoms.
- The `migrations/` folder containing existing SQLx PostgreSQL schema definitions to understand how the database is structured.

### Phase 1: Analysis & Deep Reasoning (Root-Cause Diagnosis)
Spend significant reasoning effort analyzing the workspace. You should:
1. **Trace Symptoms to Root Causes**: Read `REFACTOR.md` to understand where the developer experiences friction. Then, dive into the code to investigate *why* those friction points exist. Identify structural coupling, "stringly-typed" API bottlenecks, or data-flow issues that are the true source of the pain.
2. **Evaluate Best-Practice Alignment**: Identify architectural patterns that violate modern Rust best practices (e.g., tight coupling, excessive use of static constants instead of database entries, lack of compiler-enforced type safety).
3. **Plan Systemic Solutions**: Design database schemas, module structures, and trait boundaries that solve these issues holistically. 
4. **Embrace Structural Changes**: Accept that massive structural shifts and temporary compilation breakages are perfectly acceptable during the transition phases. Modern, idiomatic design takes precedence over immediate incremental compilation.

### Phase 2: Writing the DESIGN.md File
Create a well-structured `DESIGN.md` file in the root of the project. It must contain the following sections:

1. **Executive Summary & Modernization Vision**: A high-level overview of the target state of the codebase and why these architectural changes are necessary.
2. **Module Consolidation & Architecture**:
   - A structural map of the proposed module hierarchy.
   - Rules for module boundary isolation, specifically addressing how gaming modules should interact with core bot logic.
   - An extensibility framework blueprint for onboarding new external integrations cleanly, detailing how to isolate and structure domain data.
3. **Database & Persistence Strategy**:
   - An analysis of the current database schema (from `migrations/`).
   - A schema design and migration plan to shift hardcoded Rust constants or unstructured data into PostgreSQL.
   - Recommended SQLx patterns (e.g., compile-time query validation, transaction management, connection pooling integration).
4. **Type-Safety & Domain Modeling**:
   - Analysis of current string-based pattern matching or unstructured data models.
   - Concrete proposed Rust `enum` structures and trait implementations to enforce compile-time correctness across the domain.
5. **Architectural Trade-offs & Open Questions**:
   - Any technical risks, performance trade-offs (e.g., database queries vs. memory caching), or open questions you identify during your codebase inspection that must be resolved before executing the refactor.

### Verification
Ensure `DESIGN.md` is saved directly to the root of the workspace. The design should be detailed enough that a developer (or yourself in a subsequent step) could implement it with minimal ambiguity.