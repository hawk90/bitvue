# Design Patterns in Bitvue Project

## Comprehensive Pattern Analysis

### Summary Statistics
- **Strategy Patterns**: 15+ implementations found
- **Builder Patterns**: 10+ implementations found
- **Factory Patterns**: 8+ implementations found
- **State Patterns**: 100+ State structs/enums
- **Context Patterns**: 10 React contexts
- **Renderer Patterns**: 5+ implementations
- **Validator Patterns**: 20+ validation functions

---

## I. Strategy Pattern (15+ implementations)

### Applied Strategy Patterns

#### 1. YUV Conversion Strategy ✅
**Location**: `crates/bitvue-decode/src/strategy/mod.rs`
- `YuvConversionStrategy` - Platform-specific YUV→RGB conversion
- Implementations: AVX2, NEON, Metal, Scalar

#### 2. Workspace Rendering Strategy ✅
**Location**: `crates/ui/src/workspaces/workspace_strategy.rs`
- `ColorScheme` - Codec-specific colors (Av1, Avc, Hevc, Vvc, Mpeg2)
- `ViewRenderer` - View-specific rendering
- `PartitionRenderer` - Partition visualization
- `CodecWorkspace` - Unified codec workspace interface

#### 3. Codec Decoding Strategy ✅
**Location**: `crates/bitvue-decode/src/traits.rs`
- `Decoder` trait - Universal decoder interface
- `CodecRegistry` - Dynamic codec registration
- `DecoderFactory` - Create codec-specific decoders

### Strategy Pattern Opportunities

#### A. Parser Strategy (New)
**Problem**: Each codec has its own parsing logic
**Solution**: Unified parser interface

```rust
// crates/bitvue-codec/src/parser_strategy.rs
pub trait ParserStrategy: Send + Sync {
    fn parse_header(&mut self, data: &[u8]) -> Result<ParseResult, ParseError>;
    fn parse_frame(&mut self, data: &[u8]) -> Result<ParseResult, ParseError>;
    fn capabilities(&self) -> ParserCapabilities;
}

pub struct Av1ParserStrategy;
pub struct HevcParserStrategy;
pub struct VvcParserStrategy;
```

#### B. Comparison Strategy (New)
**Problem**: Different comparison modes mixed together
**Solution**: Separate strategies per comparison type

```rust
// crates/bitvue-core/src/compare_strategy.rs
pub trait ComparisonStrategy: Send + Sync {
    fn compare_frames(&self, a: &DecodedFrame, b: &DecodedFrame) -> ComparisonResult;
    fn compare_units(&self, a: &UnitNode, b: &UnitNode) -> ComparisonResult;
}

pub struct PsnrComparisonStrategy;
pub struct SsimComparisonStrategy;
pub struct BitwiseComparisonStrategy;
```

#### C. Validation Strategy (New)
**Problem**: Validation logic scattered across 20+ files
**Solution**: Strategy-based validation

```rust
// crates/bitvue-core/src/validation_strategy.rs
pub trait ValidationStrategy: Send + Sync {
    fn validate(&self, data: &ValidationData) -> ValidationResult;
    fn error_message(&self) -> String;
}

pub struct StrictValidationStrategy;
pub struct LenientValidationStrategy;
pub struct PermissiveValidationStrategy;
```

---

## II. Builder Pattern (10+ implementations)

### Applied Builder Patterns

#### 1. VideoFrameBuilder ✅
**Location**: `crates/bitvue-core/src/frame.rs`
- Fluent API for constructing video frames
- Validates frame dimensions

#### 2. SyntaxBuilder ✅
**Location**: `crates/bitvue-av1/src/syntax_parser/mod.rs`
- Build syntax tree model
- Tracking OBUs and syntax nodes

#### 3. Codec-Specific Frame Builders ✅
**Location**: Various codec crates
- `AvcFrameBuilder` (crates/bitvue-avc/src/frames.rs)
- `HevcFrameBuilder` (crates/bitvue-hevc/src/frames.rs)
- `Vp9FrameBuilder` (crates/bitvue-vp9/src/frames.rs)

#### 4. IVF Builder ✅
**Location**: `crates/bitvue-av1/src/ivf.rs`
- Build IVF container headers

### Builder Pattern Opportunities

#### A. Command Builder (New - HIGH PRIORITY)
**Problem**: Commands created with complex constructors
**Solution**: Builder pattern for commands

```rust
// crates/bitvue-core/src/command_builder.rs
pub struct CommandBuilder {
    command_type: Option<CommandType>,
    stream: Option<StreamId>,
    target: Option<EntityRef>,
    byte_range: Option<(u64, u64)>,
    order_type: OrderType,
    transaction_id: Option<String>,
}

impl CommandBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn select_frame(mut self, stream: StreamId, frame_key: FrameKey) -> Self {
        self.command_type = Some(CommandType::SelectFrame);
        self.stream = Some(stream);
        self.target = Some(EntityRef::Frame(frame_key));
        self
    }

    pub fn select_unit(mut self, stream: StreamId, unit_key: UnitKey) -> Self {
        self.command_type = Some(CommandType::SelectUnit);
        self.stream = Some(stream);
        self.target = Some(EntityRef::Unit(unit_key));
        self
    }

    pub fn jump_to_offset(mut self, stream: StreamId, offset: u64) -> Self {
        self.command_type = Some(CommandType::JumpToOffset);
        self.stream = Some(stream);
        self
    }

    pub fn with_byte_range(mut self, start: u64, end: u64) -> Self {
        self.byte_range = Some((start, end));
        self
    }

    pub fn with_order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = order_type;
        self
    }

    pub fn with_transaction(mut self, id: String) -> Self {
        self.transaction_id = Some(id);
        self
    }

    pub fn build(self) -> Result<Command, String> {
        let command_type = self.command_type.ok_or("Command type not set")?;
        // Validate and construct the actual Command enum
        match command_type {
            CommandType::SelectFrame => {
                let stream = self.stream.ok_or("Stream not set")?;
                let target = self.target.ok_or("Target not set")?;
                Ok(Command::SelectFrame { stream, frame_key: /* extract from target */ })
            }
            // ... other variants
        }
    }
}
```

#### B. Selection Builder (New)
**Problem**: Selection state construction is complex
**Solution**: Builder for selection state

```rust
// crates/bitvue-core/src/selection_builder.rs
pub struct SelectionBuilder {
    stream: Option<StreamId>,
    frame: Option<FrameKey>,
    unit: Option<UnitKey>,
    syntax: Option<SyntaxNodeId>,
    bit_range: Option<BitRange>,
    spatial_block: Option<SpatialBlock>,
}
```

---

## III. Factory Pattern (8+ implementations)

### Applied Factory Patterns

#### 1. DecoderFactory ✅
**Location**: `crates/bitvue-decode/src/traits.rs`
- Create codec-specific decoders
- Dynamic codec registration via CodecRegistry

#### 2. ExtractorFactory ✅
**Location**: `crates/bitvue-core/src/index_extractor.rs`
- Create index extractors for different containers

#### 3. CodecRegistry ✅
**Location**: `crates/bitvue-decode/src/traits.rs`
- Global registry for decoder factories
- Open/Closed principle implementation

### Factory Pattern Opportunities

#### A. Overlay Factory (New - HIGH PRIORITY)
**Problem**: Overlays created directly in code, hard to extend
**Solution**: Factory for overlay creation

```rust
// crates/bitvue-core/src/overlay_factory.rs
pub trait OverlayFactory: Send + Sync {
    fn create_qp_heatmap(&self) -> Box<dyn OverlayRenderer>;
    fn create_mv_overlay(&self) -> Box<dyn OverlayRenderer>;
    fn create_partition_grid(&self) -> Box<dyn OverlayRenderer>;
    fn create_diff_heatmap(&self) -> Box<dyn OverlayRenderer>;
    fn create_mode_labels(&self) -> Box<dyn OverlayRenderer>;
}

pub struct Av1OverlayFactory;
pub struct AvcOverlayFactory;
pub struct HevcOverlayFactory;
pub struct VvcOverlayFactory;

impl OverlayFactory for Av1OverlayFactory {
    fn create_qp_heatmap(&self) -> Box<dyn OverlayRenderer> {
        Box::new(Av1QpHeatmapRenderer::new())
    }
    // ... other overlays
}
```

#### B. Export Format Factory (New)
**Problem**: Export formats scattered across code
**Solution**: Centralized format factory

```rust
// crates/bitvue-core/src/export_format_factory.rs
pub trait ExportFormatFactory: Send + Sync {
    fn create_csv_exporter(&self) -> Box<dyn Exporter>;
    fn create_json_exporter(&self) -> Box<dyn Exporter>;
    fn create_json_pretty_exporter(&self) -> Box<dyn Exporter>;
    fn create_evidence_bundle_exporter(&self) -> Box<dyn Exporter>;
}
```

#### C. Context Factory (New)
**Problem**: 10+ React contexts created manually
**Solution**: Factory for context creation

```typescript
// src/contexts/ContextFactory.ts
export class ContextFactory {
    static createSelectionContext(): React.Context<SelectionState>;
    static createFrameDataContext(): React.Context<FrameDataState>;
    static createFileStateContext(): React.Context<FileState>;
    static createThemeContext(): React.Context<ThemeState>;
    // ... other contexts
}
```

---

## IV. State Pattern (100+ implementations)

### Applied State Patterns

#### 1. StreamState ✅
**Location**: `crates/bitvue-core/src/stream_state.rs`
- Manages stream lifecycle state

#### 2. TemporalState ✅
**Location**: `crates/bitvue-core/src/temporal_state.rs`
- Time-based state management

#### 3. Decoder State ✅
**Location**: `crates/bitvue-decode/src/decoder.rs`
- Decoder lifecycle states

### State Pattern Opportunities

#### A. Unified State Machine (New)
**Problem**: State logic scattered across many files
**Solution**: Unified state machine pattern

```rust
// crates/bitvue-core/src/state_machine.rs
pub trait State: Send + Sync {
    fn on_enter(&mut self);
    fn on_exit(&mut self);
    fn handle_event(&mut self, event: StateEvent) -> Transition;
}

pub enum AppState {
    Idle(IdleState),
    Loading(LoadingState),
    Decoding(DecodingState),
    Error(ErrorState),
}
```

---

## V. Context Pattern (10 React contexts)

### Applied Context Patterns

#### 1. FileStateContext ✅
**Location**: `src/contexts/FileStateContext.tsx`
- File loading and management state

#### 2. FrameDataContext ✅
**Location**: `src/contexts/FrameDataContext.tsx`
- Frame data sharing across components

#### 3. CurrentFrameContext ✅
**Location**: `src/contexts/CurrentFrameContext.tsx`
- Currently selected frame state

#### 4. LayoutContext ✅
**Location**: `src/contexts/LayoutContext.tsx`
- UI layout state

#### 5. ThumbnailContext ✅
**Location**: `src/contexts/ThumbnailContext.tsx`
- Thumbnail generation state

#### 6. SelectionContext ✅
**Location**: `src/contexts/SelectionContext.tsx`
- Tri-sync selection state

#### 7. ThemeContext ✅
**Location**: `src/contexts/ThemeContext.tsx`
- Dark/light theme state

#### 8. CompareContext ✅
**Location**: `src/contexts/CompareContext.tsx`
- Comparison workspace state

#### 9. ModeContext ✅
**Location**: `src/contexts/ModeContext.tsx`
- Player mode state

### Context Pattern Opportunities

#### A. Context Provider Factory (New)
**Problem**: Context providers manually wrapped
**Solution**: Provider factory/combinator

```typescript
// src/contexts/ContextProviders.tsx
export function createContextProvider(
    contexts: ContextConfig[]
): React.ComponentType<{ children: React.ReactNode }>;

// Usage:
const AppProvider = createContextProvider([
    { context: FileStateContext, initialState: initialFileState },
    { context: SelectionContext, initialState: initialSelectionState },
    { context: ThemeContext, initialState: initialThemeState },
]);
```

#### B. Context Selector Optimization (New)
**Problem**: Context re-renders cause performance issues
**Solution**: Selector-based context consumption

```typescript
// src/hooks/useContextSelector.ts
export function useContextSelector<T, R>(
    context: React.Context<T>,
    selector: (state: T) => R
): R;
```

---

## VI. Renderer Pattern (5+ implementations)

### Applied Renderer Patterns

#### 1. ViewRenderer ✅
**Location**: `crates/ui/src/workspaces/workspace_strategy.rs`
- View-specific rendering strategies

#### 2. PartitionRenderer ✅
**Location**: `crates/ui/src/workspaces/workspace_strategy.rs`
- Partition visualization strategies

#### 3. OverlayRenderer ✅
**Location**: `crates/ui/src/workspaces/overlays/`
- Various overlay renderers (qp.rs, mode_labels.rs, grid.rs)

### Renderer Pattern Opportunities

#### A. Unified Renderer Interface (New)
**Problem**: Renderer interfaces not consistent
**Solution**: Unified renderer trait

```rust
// crates/bitvue-core/src/renderer.rs
pub trait Renderer: Send + Sync {
    fn render(&self, ctx: &RenderContext) -> RenderResult;
    fn bounds(&self) -> Rect;
    fn intersects(&self, rect: Rect) -> bool;
}
```

---

## VII. Observer Pattern (Event System)

### Current Implementation
- Events dispatched manually in various places
- No centralized observer pattern

### Observer Pattern Opportunities

#### A. Event Observer System (New)
**Problem**: Events manually dispatched to many listeners
**Solution**: Observer pattern for events

```rust
// crates/bitvue-core/src/event_observer.rs
pub trait EventObserver: Send + Sync {
    fn on_selection_changed(&self, event: &SelectionChangedEvent);
    fn on_frame_decoded(&self, event: &FrameDecodedEvent);
    fn on_parse_complete(&self, event: &ParseCompleteEvent);
    fn on_error(&self, event: &ErrorEvent);
}

pub struct EventBus {
    observers: Vec<Box<dyn EventObserver>>,
}

impl EventBus {
    pub fn subscribe(&mut self, observer: Box<dyn EventObserver>);
    pub fn publish(&self, event: &Event);
}

// Concrete observers:
pub struct LoggingObserver;
pub struct HistoryObserver;
pub struct CacheInvalidationObserver;
pub struct AnalyticsObserver;
```

---

## VIII. Chain of Responsibility

### Current Implementation
- Command handling uses match statements

### Chain of Responsibility Opportunities

#### A. Command Processing Chain (New)
**Problem**: Command handling has complex conditional logic
**Solution**: Chain of responsibility for commands

```rust
// crates/bitvue-core/src/command_chain.rs
pub trait CommandHandler: Send + Sync {
    fn can_handle(&self, command: &Command) -> bool;
    fn handle(&mut self, command: Command) -> CommandResult;
    fn set_next(&mut self, next: Box<dyn CommandHandler>);
}

pub struct SelectionCommandHandler {
    next: Option<Box<dyn CommandHandler>>,
}

pub struct NavigationCommandHandler {
    next: Option<Box<dyn CommandHandler>>,
}

pub struct OverlayCommandHandler {
    next: Option<Box<dyn CommandHandler>>,
}

pub struct ExportCommandHandler {
    next: Option<Box<dyn CommandHandler>>,
}
```

---

## IX. Template Method Pattern

### Current Implementation
- Not explicitly applied

### Template Method Opportunities

#### A. Workspace Rendering Template (New)
**Problem**: Each workspace has similar rendering structure
**Solution**: Template method pattern

```rust
// crates/ui/src/workspaces/workspace_template.rs
pub trait WorkspaceTemplate: Send + Sync {
    fn render_header(&self, ui: &mut Ui);
    fn render_toolbar(&self, ui: &mut Ui);
    fn render_content(&self, ui: &mut Ui);
    fn render_footer(&self, ui: &mut Ui);

    fn show(&mut self, ui: &mut Ui) {
        self.render_header(ui);
        self.render_toolbar(ui);
        ui.separator();
        self.render_content(ui);
        self.render_footer(ui);
    }
}
```

---

## X. React/TypeScript Patterns (108 files)

### Applied Patterns

#### 1. React.memo ✅
**Location**: 108+ files
- Performance optimization for components
- Examples: `Filmstrip.tsx`, `Timeline.tsx`, various panels

#### 2. useMemo/useCallback ✅
**Location**: 108+ files
- Memoized values and callbacks
- Prevents unnecessary re-renders

#### 3. Custom Hooks ✅
**Location**: `src/hooks/`
- `useFileOperations`
- `useThumbnail`
- `useCanvasInteraction`
- `useDropdown`

### React Pattern Opportunities

#### A. Compound Component Pattern (New)
**Problem**: Complex components with many props
**Solution**: Compound components

```typescript
// Example for Timeline:
<Timeline>
  <Timeline.Ruler />
  <Timeline.Filmstrip />
  <Timeline.Markers />
  <Timeline.Controls />
</Timeline>
```

#### B. Render Props Pattern (New)
**Problem**: Shared logic across components
**Solution**: Render props

```typescript
// Example:
<DataLoader url="/api/frames">
  {({ data, loading, error }) => (
    <Filmstrip data={data} loading={loading} error={error} />
  )}
</DataLoader>
```

---

## Implementation Priority

### Phase 1: Immediate Value (Quick Wins)
1. **Command Builder** - Reduces command construction complexity
2. **Overlay Factory** - Unifies overlay creation logic
3. **Selection Builder** - Simplifies selection state construction

### Phase 2: High Impact
1. **Parser Strategy** - Unified parsing interface for codecs
2. **Comparison Strategy** - Cleaner comparison logic
3. **Export Format Factory** - Centralized export logic
4. **Event Observer System** - Better event handling

### Phase 3: Medium Value
1. **Validation Strategy** - Consistent validation across codebase
2. **Command Processing Chain** - Cleaner command flow
3. **Context Factory** - Simplified context creation
4. **Unified State Machine** - Centralized state management

### Phase 4: Nice to Have
1. **Workspace Template Method** - Consistent workspace structure
2. **Compound Components** - Cleaner component API
3. **Renderer Interface** - Unified rendering

---

## Examples of Current Anti-Patterns

### Anti-Pattern 1: Direct Construction Instead of Builder

**Current**:
```rust
let command = Command::SelectFrame {
    stream: StreamId::A,
    frame_key: FrameKey { index: 42 },
};
```

**Improved**:
```rust
let command = CommandBuilder::new()
    .select_frame(StreamId::A, FrameKey::new(42))
    .build()?;
```

### Anti-Pattern 2: Switch/Match Instead of Strategy

**Current**:
```rust
match comparison_type {
    ComparisonType::PSNR => calculate_psnr(a, b),
    ComparisonType::SSIM => calculate_ssim(a, b),
    ComparisonType::Bitwise => compare_bitwise(a, b),
}
```

**Improved**:
```rust
let strategy: Box<dyn ComparisonStrategy> = ComparisonStrategyFactory::create(comparison_type)?;
strategy.compare_frames(a, b)
```

### Anti-Pattern 3: Direct Conditional Instead of Strategy

**Current**:
```rust
if codec == CodecType::AV1 {
    // AV1-specific parsing
} else if codec == CodecType::HEVC {
    // HEVC-specific parsing
}
```

**Improved**:
```rust
let parser: Box<dyn ParserStrategy> = ParserFactory::create(codec)?;
let result = parser.parse_frame(data)?;
```

### Anti-Pattern 4: Manual Event Dispatching

**Current**:
```rust
// Manually call each listener
on_selection_changed(&event);
on_history_updated(&event);
on_cache_invalidated(&event);
```

**Improved**:
```rust
event_bus.publish(SelectionChangedEvent { /* ... */ });
// All subscribed observers receive the event automatically
```

---

## Recommended Next Steps

1. **Start with Command Builder** (Phase 1)
   - High impact, low risk
   - Immediate value across entire codebase
   - Reduces command construction boilerplate

2. **Add Overlay Factory** (Phase 1)
   - Unifies overlay creation logic
   - Makes adding new overlays easier
   - Codec-specific overlay factories

3. **Implement Parser Strategy** (Phase 2)
   - Unified parsing interface for all codecs
   - Easier to add new codec parsers
   - Cleaner separation of concerns

4. **Add Comparison Strategy** (Phase 2)
   - Cleans up comparison logic
   - Easier to add new comparison metrics
   - Better testability

5. **Build Event Observer System** (Phase 2)
   - Better event handling
   - Decouples event producers from consumers
   - Easier to add new event listeners

Would you like me to implement any of these patterns?
