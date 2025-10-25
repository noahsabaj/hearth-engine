# Hearth Engine - Claude Instructions

Emojis are explicitly forbidden to be used anywhere and at any point in the codebase. Enforce this.

**PURPOSE**: This document contains TIMELESS instructions.

## ENVIRONMENT SETUP
- **Claude (AI)**: Working in Linux Mint at `/home/nsabaj/Code/hearth-workspace/hearth-engine`

## PROJECT OVERVIEW
This is **Hearth Engine** - a frontier SOTA voxel game engine. We are building in this order:
1. **ENGINE** (Current Phase) - Game-agnostic voxel engine with cutting-edge performance
2. **GAME** (Semi-Current (game is a demonstration of the engine) Phase) - Game implementation using the engine
3. **FRAMEWORK** (Final Phase) - Tools enabling others to build ANY voxel game

### Game Vision (Future Implementation)
**Core Foundation: Emergent Gameplay from Physics Simulation**
- **1dcm³ voxels** (10cm cubes) - perfect scale for realistic destruction and structural physics
- **Everything transformable** - Every voxel can be destroyed/built/changed with realistic physics
- **Full voxel physics simulation** - Thermal, fluid, sound, structural integrity per voxel
- **Stone age → space age progression** - intuitive discovery through experimentation
- **Planetary servers** - each region is its own planet that develops unique culture
- **"Real life but fun"** - Complex emergent behavior from simple physics laws
- **Target**: 10,000+ concurrent players per planet at 144+ FPS

**Emergent Possibilities** (not forced, but enabled by the physics):
- **Physical information economy** - knowledge could spread through hand-copied books
- **Local communication** - voice could carry realistically through physics simulation  
- **Player-created civilizations** - cities with realistic construction and collapse
- **Technology discovery** - radio, printing, metallurgy discovered through experimentation
- **Complete sandbox freedom** - roaming bandits, isolated hermits, anything emergent

Think "Teardown" destruction physics meets "Minecraft" building meets "Garry's Mod" emergent creativity:
- Rust-like survival and realistic raiding (walls actually collapse)
- SimCity urban planning with structural engineering constraints
- DayZ realism with physics-based sound propagation
- EVE Online emergent politics from resource scarcity
- All possible simultaneously in one persistent world

The engine provides realistic physics. Players discover what's possible.

## CRITICAL PHILOSOPHY
**DATA-ORIENTED PROGRAMMING ONLY** - This codebase follows strict DOP principles:
- NO classes, objects, or OOP patterns
- NO methods - only functions that transform data
- Data lives in shared buffers (WorldBuffer, RenderBuffer, etc.)
- Systems are stateless kernels that read/write buffers
- GPU-first architecture - data lives where it's processed
- If you're writing `self.method()`, you're doing it wrong

## CRITICAL FACTS - NEVER FORGET

### CHUNK SIZE IS 50
- **The chunk size is 50x50x50 voxels** (5m x 5m x 5m with 10cm voxels)
- **Use CHUNK_SIZE constant from constants.rs, which is set to 50**
- **This applies to BOTH engine and game code**

### DO NOT USE include! STATEMENTS
- **NEVER use `include!("../../constants.rs")` or similar**
- **Use proper module imports instead: `use crate::constants;`**
- **include! is a code smell and breaks proper module structure**
- **Each crate should have its own constants module if needed**

### Constants Usage Pattern
```rust
// WRONG - Never do this!
include!("../../constants.rs");

// CORRECT - Use proper imports
use crate::constants::core::CHUNK_SIZE;
// or for games:
use crate::constants::world::CHUNK_SIZE;
```

## WORKFLOW REQUIREMENTS

### 1. Documentation Updates

#### Documentation Principles
- **Single Source of Truth** - Each fact lives in exactly ONE document
- **No Duplication** - Reference other docs, don't copy content
- **Timeless vs Temporal** - CLAUDE.md has timeless info
- **Regular Reviews** - Check for stale info, consolidation opportunities
- **Honest Metrics** - Real percentages, not optimistic guesses

### 2. Verification Process
Before considering ANY task complete:
1. Run `cargo check` - must pass
2. Run `cargo test` - must pass
3. Run `cargo clippy` - address warnings
4. Verify the feature actually works as intended
5. Update all relevant documentation
6. Check that no unwrap() calls were added
7. Ensure no OOP patterns were introduced

## CODE STANDARDS

### Long-Term Code Philosophy
- **NO BANDAIDS** - No temporary solutions, hacks, or "fix it later" code
- **I Trust You** - What does that mean to you?
- **Build for decades** - This code should decrease technical debt over time
- **Clean code when possible** - Readability matters, but not at the cost of performance
- **Simple > Complex** - When in doubt, simple is your route, but be realistic, some code can't help but be complex. Aim for simple
- **Kaizen over revolution** - Continuous small improvements over big rewrites
- **Think in systems** - How will this code interact with features we haven't built yet?
- **Measure twice, code once** - Design decisions should be deliberate and documented

### Engineering Principles (Data-Oriented Style)

#### Single Source of Truth
- Each piece of data lives in exactly ONE buffer
- Each fact documented in exactly ONE place
- No duplicated state between systems
- Reference, don't copy

#### DRY (Don't Repeat Yourself) - DOP Version
- Reuse data transformations, not object hierarchies
- Generic kernels over specialized systems
- Shared buffers over message passing
- If you're copying code, you're missing a pattern

#### KISS (Keep It Simple) - But Fast
- Simple data layouts → complex emergent behavior
- One buffer + one kernel > Ten interacting objects
- Prefer arrays over complex data structures
- The fastest code is code that doesn't exist

#### YAGNI (You Aren't Gonna Need It)
- Don't add buffers "for future use"
- Don't create abstractions without 3+ use cases
- Don't optimize without profiling
- Today's requirements only (but done right)

#### Principle of Least Surprise
- Voxels are always 1dcm³ = 10cm cubes (no exceptions)
- Errors always bubble up (no silent failures)
- Data flows one direction (no backchannels)
- Names mean exactly what they say

#### Separation of Concerns - Data Style
- Each buffer has ONE purpose
- Each kernel does ONE transformation
- Physics doesn't know about rendering
- Network doesn't know about gameplay

#### Design by Contract
- Functions document their data requirements
- Preconditions checked in debug (assumed in release)
- Buffer layouts are contracts between systems
- Breaking changes require version bumps

#### Fail Fast, Fail Loud
- Invalid states panic in debug builds
- Errors propagate immediately (no error hiding)
- Assumptions documented and verified
- Better to crash in dev than corrupt in prod

#### Make It Work, Make It Right, Make It Fast
1. **Work**: Get the feature functioning (even if slow)
2. **Right**: Clean up the data flow, remove hacks
3. **Fast**: Profile and optimize based on evidence
- Never skip step 2 to get to step 3

#### The Boy Scout Rule
- Leave code better than you found it
- Fix neighboring unwraps when touching a file
- Update stale comments as you go
- Small improvements compound over time

### Error Handling
- NEVER use `.unwrap()` - use `?` operator or proper error handling
- Create module-specific error types (NetworkError, RenderError, etc.)
- Every fallible operation must return Result<T, E>

### Data Layout
```rust
// WRONG - OOP style
struct Chunk {
    fn generate(&mut self) { } // NO METHODS!
}

// CORRECT - DOP style
struct ChunkData {
    voxels: Buffer<Voxel>,
    position: ChunkPos,
}

fn generate_chunk(data: &mut ChunkData, gen_params: &GenParams) {
    // Transform data, no self
}
```

### Performance
- Profile before optimizing
- Data locality matters more than "clean" code
- Prefer SOA (Structure of Arrays) over AOS
- Batch operations for GPU

## EVERGREEN PRIORITIES (Always True)
1. **Zero-panic architecture** - No unwrap(), no crashes in production
2. **Data-oriented design** - Everything is data + kernels (DOP), no OOP
3. **GPU-first computation** - Always ask "can GPU do this?"
4. **Documentation accuracy** - Keep all docs in sync with reality
5. **Long-term thinking** - No bandaids, build for decades

## COMMON PITFALLS TO AVOID
1. **Creating unnecessary documents** - Fix code first, document after
2. **OOP creep** - Watch for methods, traits with behavior, unnecessary abstractions
3. **CPU-thinking** - Always ask "can GPU do this in parallel?"

## TESTING REMINDERS
- Test with missing files (no unwrap crashes)
- Test network disconnections (graceful handling)
- Test malformed data (proper errors, not panics)
- Benchmark everything - we need 1000x performance

## CRITICAL: CLAIMS VS REALITY PREVENTION
**Hearth Engine Specific**: This project has experienced severe "claims vs reality" gaps where agents report success without verification.

### Four Root Causes of False Claims (Identified in Sprint 36):
1. **Overconfidence Bias** - Assuming unwrap() elimination worked without checking rg counts
2. **Lack of Verification** - Not running `cargo check`, `cargo test` to prove claims
3. **Text Generation vs Reality** - Generating "mission accomplished" text without evidence
4. **Speed over Accuracy** - Rushing to claim sprint completion rather than verify
5. **AI-Sycophancy** - The AI being overly nice to the point of letting the user make a preventable mistake, this is bad,, the AI must always be neutral and unbiased in order to be most effective

### Hearth Engine Verification Requirements:
**MANDATORY FOR ALL WORK:**
- `cargo check --lib` MUST pass before claiming compilation success
- `cargo test --lib` MUST be attempted before claiming functionality
- `rg "\.unwrap\(\)" src --type rust -c` MUST be run to verify unwrap elimination
- `cargo run --example engine_testbed` MUST be tested for user experience
- All metrics must be measured, never estimated

### Hearth Engine Specific Checks:
- **Player Movement**: Verify WASD keys work with actual testing
- **Spawn System**: Verify safe spawn with actual gameplay
- **Save/Load**: Verify no corruption with actual save/load cycles
- **Performance**: Verify claims with actual benchmarks
- **DOP Conversion**: Verify allocation counts with profiling

### Language for Hearth Engine:
- "Zero unwraps achieved" → ✅ "Targeted unwraps, running rg count..."
- "Sprint 36 complete" → ✅ "Sprint 36 changes made, running QA..."
- "Player movement fixed" → ✅ "Modified movement code, testing WASD..."
-  "Engine ready for production" → ✅ "Engine compiles, testing functionality..."

**Hearth Engine Principle**: Show, don't tell. Prove, don't claim.

## VISION REMINDERS
This engine enables the ultimate voxel-based games:
- **"Real life but fun"** - Complex societies emerge from simple physics
- **Every voxel transformable** - Build cities, destroy mountains, redirect rivers
- **No forced gameplay** - Those "1000 players simulate civilization" videos? Just ONE way to play
- **Player-created everything**:
  - Cities with actual smithies, taverns, mayors (not NPCs - real players)
  - Confederacies of villages with complex politics
  - Roaming bandit clans with hideouts
  - Hermit scholars preserving ancient knowledge
  - Trade empires spanning continents
- **Communication mirrors reality**:
  - Voice propagates through actual physics - not arbitrary distance
  - Sound waves blocked by walls, carried by tunnels, echo in caves
  - Material acoustics matter - concrete muffles, wood transmits
  - Stealth is physical - footsteps on stone vs grass
  - No global chat - messages travel physically
  - Write signs, books, letters - all hand-copied
  - Dye and sew fabric for banners, flags, uniforms
  - Late-game radio tech enables long-distance comms
- **Visual identity emerges**:
  - Nations design their own flags from dyed fabric
  - Guilds create banners to mark territory
  - Shops hang signs with hand-written text
  - All player-created, no presets
- **Blend of the best**:
  - Minecraft's building and exploration
  - Rust's survival and raiding
  - Garry's Mod's creative emergence
  - EVE Online's player politics
  - SimCity's urban planning
- **Full physics simulation per voxel**:
  - Thermal dynamics - heat spreads, fire propagates
  - Fluid dynamics - water flows, pressure matters  
  - Sound physics - waves propagate, materials absorb/reflect
  - Structural integrity - buildings can collapse
  - Material properties - density, conductivity, acoustics
- **Physics enables stories**:
  - Build soundproof rooms for secret meetings
  - Castle walls block both arrows AND eavesdroppers
  - Footsteps on stone alert guards
  - Caves carry whispers for kilometers
  - Thunder echoes off mountains
- **Knowledge has weight** - Technologies spread through teaching, not wikis

The engine is the laws of physics. Players are the force of history.

Every line of code should enable player creativity, not constrain it. Remember: we're building a game-agnostic ENGINE first.

## WHEN IN DOUBT
1. Choose performance over "clean" code
2. Choose data-oriented over object-oriented
3. Choose GPU computation over CPU
4. Choose explicit over abstract
5. Choose measured results over assumptions
6. Choose boring stability over exciting features
7. Choose proven patterns over novel experiments
8. Choose cache-friendly over "logical" organization
9. Choose batch operations over individual updates
10. Choose predictable behavior over clever tricks
11. Choose documentation over self-documenting code
12. Choose tomorrow's maintainability over today's convenience
13. Choose real benchmarks over theoretical benefits
14. Choose user trust over feature count
15. Choose physics accuracy over gameplay shortcuts