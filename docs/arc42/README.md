# BrokerX Architecture Documentation

This directory contains the complete architecture documentation for the BrokerX system, following the arc42 template.

## Files Overview

- **`docs.md`** - Main arc42 documentation with all architectural views
- **`level1_context.pintora`** - System context diagram (Level 1)
- **`level2_domain.pintora`** - Domain architecture diagram (Level 2)  
- **`level3_ports_adapters.pintora`** - Ports and adapters diagram (Level 3)
- **`level4_modules.pintora`** - Module structure diagram (Level 4)
- **`order_processing_detail.pintora`** - Detailed order processing sequence diagram
- **`activity.pintora`** - Business activity diagram
- **`activity.svg`** - Generated activity diagram

## Architecture Levels

### Level 1: System Context
Shows the external boundaries of the BrokerX system:
- **Web Browser** (primary actor)
- **PostgreSQL** (data persistence)
- **SMTP Server** (email notifications)

### Level 2: Domain Architecture  
Shows the internal organization of the domain:
- **BrokerX** aggregate coordinating all operations
- **ProcessingPool** with **SharedState** for async order processing
- **UserRepo** and **OrderRepo** for data access
- **PreTradeValidator** for risk validation
- **MfaService** for multi-factor authentication

### Level 3: Ports and Adapters
Shows the hexagonal architecture implementation:
- **Primary ports**: Web handlers, CLI interfaces
- **Secondary ports**: Repository interfaces, MFA interfaces
- **Adapters**: Database, MFA, and in-memory implementations

### Level 4: Module Structure
Shows the Rust crate organization:
- **domain**: Pure business logic
- **app**: Web application and orchestration
- **database_adapter**: PostgreSQL persistence
- **mfa_adapter**: Multi-factor authentication services
- **in_memory_adapter**: Test implementations

### Order Processing Detail
Sequence diagram showing the complete order lifecycle:
1. Web form submission and validation
2. Pre-trade risk checks
3. Order creation and queuing
4. Asynchronous processing by worker threads
5. Status transitions (Queued → Pending → Filled/Rejected)
6. Portfolio updates

## Diagram Format

All diagrams use the Pintora format, which can be rendered through:
- **Typst** documents using the `pintorita` package
- **VS Code** with Pintora extensions
- **Online** Pintora renderers

## Keeping Documentation Current

The architecture diagrams are code-driven and reflect the actual implementation. When making changes to the codebase:

1. Update the relevant Pintora diagrams
2. Verify diagrams match the current code structure
3. Update descriptions in `docs.md` if needed
4. Consider adding new diagrams for significant architectural changes

## Architecture Decisions

Architectural decisions are documented in the `../adr/` directory as ADRs (Architecture Decision Records).
