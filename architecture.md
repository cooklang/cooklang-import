# Architecture Overview

This document provides a high-level overview of the cooklang-import system architecture, showing the different input flows and their outcomes.

## System Architecture

```mermaid
flowchart TB
    %% Input Sources
    URL[URL Input]
    TEXT[Text Input]
    IMAGE[Image Input]

    %% Processing Stages
    FETCH[Fetch Webpage]
    EXTRACT[Extraction Pipeline<br/>JSON-LD → MicroData →<br/>HTML Class → Plain Text LLM]
    OCR[OCR Processing<br/>Google Cloud Vision]

    %% Intermediate State
    RECIPE_DATA[Recipe Data<br/>Ingredients + Instructions]

    %% Configuration
    CONFIG[Configuration<br/>config.toml + env vars]

    %% LLM Conversion
    LLM[LLM Conversion<br/>OpenAI | Anthropic | Google<br/>Azure OpenAI | Ollama]

    %% Output Modes
    RECIPE_OUT[Recipe Struct<br/>Markdown Format]
    COOKLANG_OUT[Cooklang Format<br/>With Frontmatter]

    %% Flow connections
    URL --> FETCH
    FETCH --> EXTRACT
    EXTRACT --> RECIPE_DATA

    TEXT --> RECIPE_DATA

    IMAGE --> OCR
    OCR --> RECIPE_DATA

    RECIPE_DATA --> |extract_only mode| RECIPE_OUT
    RECIPE_DATA --> |default mode| LLM

    CONFIG -.-> LLM
    LLM --> COOKLANG_OUT

    %% Styling
    classDef inputStyle fill:#e1f5ff,stroke:#01579b,stroke-width:2px
    classDef processStyle fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef outputStyle fill:#e8f5e9,stroke:#1b5e20,stroke-width:2px
    classDef configStyle fill:#f3e5f5,stroke:#4a148c,stroke-width:2px

    class URL,TEXT,IMAGE inputStyle
    class FETCH,EXTRACT,OCR,LLM processStyle
    class RECIPE_OUT,COOKLANG_OUT outputStyle
    class CONFIG configStyle
```

## Input Flows

### 1. URL → Recipe/Cooklang
The most common use case where a recipe URL is provided:
- Fetches the webpage content
- Runs through extraction pipeline (tries multiple strategies in order)
- Returns either raw Recipe struct or converts to Cooklang format

### 2. Text → Cooklang
For plain text recipes without structure:
- Accepts unstructured recipe text
- Uses LLM to parse and convert directly to Cooklang format
- Cannot use `extract_only` mode (no structured extraction)

### 3. Image → Cooklang
For recipe images (photos, screenshots):
- Uses Google Cloud Vision API to perform OCR
- Extracted text is treated as plain text
- Converts to Cooklang format using LLM
- Cannot use `extract_only` mode (requires OCR and parsing)

## Processing Components

### Extraction Pipeline
Attempts multiple extraction strategies in order of reliability:
1. **JSON-LD** - Structured recipe data in `<script type="application/ld+json">`
2. **MicroData** - HTML5 microdata attributes
3. **HTML Class** - Common CSS class patterns (e.g., `.ingredient`, `.instruction`)
4. **Plain Text LLM** - Last resort fallback using LLM to extract from page text

### LLM Conversion
Converts recipe content to Cooklang format:
- Supports multiple providers with automatic fallback
- Configurable via `config.toml` or environment variables
- Generates frontmatter from recipe metadata

## Output Modes

### Recipe Struct (extract_only)
Returns structured recipe data without conversion:
- Ingredients and instructions in markdown format
- Metadata (cook time, servings, etc.)
- Source URL

### Cooklang Format (default)
Converts recipe to Cooklang syntax:
- YAML frontmatter with metadata
- Ingredients marked with `@` syntax
- Cookware marked with `#` syntax
- Timers marked with `~` syntax

## Configuration

The system loads configuration from multiple sources (in priority order):
1. Environment variables (e.g., `OPENAI_API_KEY`, `COOKLANG__PROVIDERS__OPENAI__MODEL`)
2. `config.toml` file in current directory
3. Default values

This allows flexibility from simple environment-only setup to complex multi-provider configurations.
