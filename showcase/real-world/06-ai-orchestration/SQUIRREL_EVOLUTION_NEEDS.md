# 🐿️ Squirrel Evolution Needs

**Date:** December 8, 2025  
**Context:** Making Squirrel fully support agnostic AI capability routing  
**Status:** Gap Analysis Complete  

---

## 🎯 What Squirrel Already Has ✅

### **1. Capability Infrastructure** ✅

```rust
// squirrel/crates/main/src/capability_registry.rs
pub struct CapabilityRegistry {
    capabilities: HashMap<String, Vec<RegisteredPrimal>>,
    // ...
}

impl CapabilityRegistry {
    pub async fn discover_by_capability(&self, capability: &PrimalCapability) -> Result<Vec<RegisteredPrimal>> {
        // Runtime capability discovery
    }
}
```

**What This Means:**
- ✅ Runtime capability discovery
- ✅ Dynamic provider registration
- ✅ No hardcoded primal names

### **2. AI Tools Framework** ✅

```rust
// squirrel/crates/tools/ai-tools/src/common/capability/mod.rs
pub enum TaskType {
    TextGeneration,
    ImageGeneration,  // ← We need this!
    Embedding,
    // ...
}
```

**What This Means:**
- ✅ Image generation capability type exists
- ✅ Universal AI provider architecture
- ✅ Capability discovery engine

### **3. API Server Infrastructure** ✅

```rust
// squirrel/crates/main/src/api/server.rs
pub struct ApiServer {
    // HTTP API with warp
    // Endpoints for health, metrics, ecosystem
}
```

**What This Means:**
- ✅ HTTP server exists
- ✅ Ecosystem integration
- ⚠️  Missing AI execution endpoints

---

## ❌ What Squirrel Needs to Evolve

### **Gap 1: AI Execution API Endpoints** ⚠️

**Missing:**
```rust
// Need to add these endpoints to API server

#[post("/ai/execute")]
async fn execute_ai_request(request: AiRequest) -> Result<AiResponse>

#[post("/ai/generate-image")]
async fn generate_image(request: ImageRequest) -> Result<ImageResponse>

#[post("/ai/generate-text")]
async fn generate_text(request: TextRequest) -> Result<TextResponse>

#[post("/ai/query-capabilities")]
async fn query_capabilities(request: CapabilityQuery) -> Result<Vec<Provider>>
```

**Why This Matters:**
- Demo needs `/ai/generate-image` endpoint
- Clients need unified AI execution interface
- Capability-based routing needs API exposure

---

### **Gap 2: Provider Adapter Layer** ⚠️

**What Exists:**
```rust
// ai-tools has provider abstractions
pub trait AIClient {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
}
```

**What's Missing:**
```rust
// Need image generation adapters

pub trait ImageGenerationProvider {
    async fn generate_image(&self, request: ImageRequest) -> Result<ImageResponse>;
}

// Implementations:
- OpenAIImageProvider (DALL-E)
- HuggingFaceImageProvider (Stable Diffusion)
- LocalImageProvider (local models)
```

**Why This Matters:**
- Normalize responses from different providers
- Handle provider-specific quirks (HuggingFace router, OpenAI formats)
- Enable fallback and retry logic

---

### **Gap 3: Environment-Based Provider Discovery** ⚠️

**What Exists:**
```rust
// ai-tools/src/local/universal_provider/universal/discovery.rs
pub struct CapabilityDiscoveryEngine {
    // Discovers from environment, network, filesystem
}
```

**What's Missing:**
- Integration between discovery engine and API server
- Provider registration on startup
- Capability refresh mechanism

**Needed:**
```rust
// On Squirrel startup
async fn initialize_providers() {
    // 1. Load environment variables (CAPABILITY_IMAGE_GENERATION_*)
    // 2. Discover available providers
    // 3. Register with capability registry
    // 4. Start heartbeat monitoring
}
```

**Why This Matters:**
- Providers need to be discovered before first request
- Environment config (squirrel-image-providers.env) needs to be loaded
- Dynamic provider addition/removal

---

### **Gap 4: Request/Response Normalization** ⚠️

**Challenge:**
Different providers have different formats:

```rust
// OpenAI format
{
  "prompt": "...",
  "size": "512x512",
  "n": 1
}
→ returns: {"data": [{"url": "..."}]}

// HuggingFace format
{
  "inputs": "..."
}
→ returns: binary image data

// Need normalization
pub async fn normalize_image_request(
    capability_request: ImageRequest,
    provider: &Provider,
) -> ProviderSpecificRequest {
    match provider.vendor {
        "openai" => map_to_openai_format(capability_request),
        "huggingface" => map_to_huggingface_format(capability_request),
        _ => generic_format(capability_request),
    }
}
```

**Why This Matters:**
- Demo sends one format
- Providers expect different formats
- Responses come back in different formats
- Squirrel needs to normalize everything

---

### **Gap 5: Provider Selection Logic** ⚠️

**What's Needed:**
```rust
pub struct ProviderSelector {
    registry: Arc<CapabilityRegistry>,
}

impl ProviderSelector {
    pub async fn select_best_provider(
        &self,
        capability: &str,
        requirements: &Requirements,
    ) -> Result<Provider> {
        // 1. Query registry for providers matching capability
        let providers = self.registry
            .discover_by_capability(capability)
            .await?;
        
        // 2. Score providers based on requirements
        let scored = self.score_providers(&providers, requirements);
        
        // 3. Select best match
        let best = scored.first()
            .ok_or(Error::NoProviderAvailable)?;
        
        // 4. Check health
        if !best.is_healthy {
            // Try fallback
        }
        
        Ok(best.clone())
    }
    
    fn score_providers(
        &self,
        providers: &[Provider],
        requirements: &Requirements,
    ) -> Vec<(Provider, f64)> {
        providers.iter().map(|p| {
            let mut score = 0.0;
            
            // Quality match
            if requirements.quality == "high" && p.quality == "high" {
                score += 10.0;
            }
            
            // Cost preference
            if requirements.cost_preference == "optimize" {
                score += (1.0 - p.cost) * 5.0;
            }
            
            // Latency
            score += (1.0 / p.avg_latency_ms as f64) * 1000.0;
            
            // Reliability
            score += p.reliability * 5.0;
            
            (p.clone(), score)
        }).sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
          .collect()
    }
}
```

**Why This Matters:**
- Automatic best-provider selection
- Cost/quality/latency optimization
- Graceful fallback

---

### **Gap 6: Songbird Integration** ⚠️

**What's Needed:**
```rust
// On startup, register Squirrel with Songbird
async fn register_with_songbird() {
    let songbird_client = SongbirdClient::new("http://localhost:8080");
    
    // Register Squirrel as AI capability provider
    songbird_client.register_service({
        service_id: "squirrel-ai-gateway",
        capabilities: [
            "ai.text.generation",
            "ai.image.generation",
            "ai.embedding",
        ],
        endpoint: "http://localhost:9090",
        health_check: "/health",
    }).await?;
    
    // Start heartbeat
    tokio::spawn(async move {
        loop {
            songbird_client.heartbeat().await;
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

// Query Songbird for other AI services
async fn query_songbird_for_providers(capability: &str) -> Result<Vec<ServiceInfo>> {
    songbird_client
        .query_services_by_capability(capability)
        .await
}
```

**Why This Matters:**
- Squirrel registers its capabilities with Songbird
- Other services can discover Squirrel's AI capabilities
- Distributed provider discovery across the mesh

---

## 📊 Priority Order

### **Phase 1: Core API Endpoints** (High Priority) 🔴

1. Add `/ai/generate-image` endpoint
2. Add `/ai/generate-text` endpoint
3. Add `/api/v1/capabilities` query endpoint

**Impact:** Demo will work, basic capability routing functional

**Time:** 2-4 hours

---

### **Phase 2: Provider Discovery** (High Priority) 🔴

1. Load `squirrel-image-providers.env` on startup
2. Parse `CAPABILITY_*` environment variables
3. Register providers in capability registry
4. Start capability refresh timer

**Impact:** Providers discovered at runtime, no hardcoding

**Time:** 2-3 hours

---

### **Phase 3: Request Normalization** (Medium Priority) 🟡

1. Implement provider-specific request mapping
2. Implement provider-specific response parsing
3. Add error handling and retry logic

**Impact:** Works with real providers (OpenAI, HuggingFace)

**Time:** 3-4 hours

---

### **Phase 4: Provider Selection** (Medium Priority) 🟡

1. Implement scoring algorithm
2. Add fallback logic
3. Add health checking
4. Add load balancing

**Impact:** Intelligent provider selection, automatic optimization

**Time:** 2-3 hours

---

### **Phase 5: Songbird Integration** (Low Priority) 🟢

1. Register Squirrel with Songbird on startup
2. Query Songbird for distributed providers
3. Heartbeat and health monitoring
4. Service mesh coordination

**Impact:** Full distributed AI orchestration

**Time:** 3-4 hours

---

## 🎯 Minimum Viable Evolution (Phase 1-2)

To get the demo working and validate architecture:

**Must Have:**
1. ✅ `/ai/generate-image` API endpoint
2. ✅ Environment-based provider discovery
3. ✅ Basic request routing
4. ✅ OpenAI adapter (since it works now)

**Nice to Have:**
- HuggingFace adapter (for free tier)
- Provider selection logic
- Error handling

**Can Wait:**
- Songbird integration
- Advanced fallback
- Load balancing

---

## 🛠️ Implementation Guide

### **Step 1: Add AI API Module**

```rust
// squirrel/crates/main/src/api/ai.rs

use warp::Filter;
use std::sync::Arc;
use crate::ai_tools::UniversalAIProvider;

pub fn ai_routes(
    provider: Arc<UniversalAIProvider>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    generate_image(provider.clone())
        .or(generate_text(provider.clone()))
        .or(query_capabilities(provider))
}

fn generate_image(
    provider: Arc<UniversalAIProvider>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("ai" / "generate-image")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_provider(provider))
        .and_then(handle_generate_image)
}

async fn handle_generate_image(
    request: ImageGenerationRequest,
    provider: Arc<UniversalAIProvider>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // 1. Convert request to capability query
    let capability_request = AiRequest {
        capability: RequestedCapability::ImageGeneration {
            resolution: request.size,
            quality: request.quality_preference,
        },
        input: json!({"prompt": request.prompt}),
        params: request.params,
    };
    
    // 2. Execute via universal provider
    let response = provider
        .execute(capability_request)
        .await
        .map_err(|e| warp::reject::custom(ApiError::from(e)))?;
    
    // 3. Return normalized response
    Ok(warp::reply::json(&ImageGenerationResponse {
        image_url: response.output["url"].as_str().unwrap().to_string(),
        provider_id: response.metadata.provider_id,
        cost: response.metadata.cost_usd,
        latency_ms: response.metadata.latency_ms,
    }))
}
```

### **Step 2: Update Server to Include AI Routes**

```rust
// squirrel/crates/main/src/api/server.rs

mod ai;  // Add this

impl ApiServer {
    pub async fn start(&self) -> Result<()> {
        // ... existing code ...
        
        // Add AI routes
        let ai_provider = Arc::new(UniversalAIProvider::new(
            config.ai_provider_config
        ).await?);
        
        let ai_routes = ai::ai_routes(ai_provider);
        
        let routes = health_routes
            .or(ecosystem_routes)
            .or(metrics_routes)
            .or(ai_routes);  // ← Add this
        
        // ... rest of server startup ...
    }
}
```

### **Step 3: Load Environment on Startup**

```rust
// squirrel/crates/main/src/main.rs or startup code

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    if let Ok(env_file) = std::env::var("SQUIRREL_PROVIDERS_ENV") {
        dotenv::from_filename(&env_file).ok();
    }
    
    // Initialize AI provider with discovery
    let ai_config = UniversalAIConfig {
        discovery: CapabilityDiscoveryConfig {
            methods: vec![
                DiscoveryMethod::Environment { prefix: "CAPABILITY_" },
            ],
            refresh_interval_ms: 60000,
        },
    };
    
    let ai_provider = UniversalAIProvider::new(ai_config).await?;
    
    // ... start API server with ai_provider ...
}
```

---

## ✅ Success Criteria

### **When Evolution is Complete:**

1. **Demo Works** ✅
   ```bash
   ./demo-agnostic-image.sh
   # Returns: Image generated via Squirrel!
   ```

2. **Providers Discovered** ✅
   ```bash
   curl http://localhost:9090/api/v1/capabilities
   # Returns: [{"type": "image.generation", "providers": [...]}]
   ```

3. **Zero Hardcoding** ✅
   - Demo doesn't know about vendors
   - Squirrel core doesn't know about vendors
   - Only config knows about vendors

4. **Router Changes Transparent** ✅
   - HuggingFace changes router
   - Update `squirrel-image-providers.env`
   - Everything else unchanged

---

## 📝 Summary

### **What Squirrel Has** ✅
- Capability registry infrastructure
- AI tools framework
- HTTP API server
- Ecosystem integration

### **What Squirrel Needs** ⚠️
1. AI execution API endpoints (`/ai/generate-image`, etc.)
2. Environment-based provider discovery integration
3. Request/response normalization layer
4. Provider selection logic
5. Songbird service registration

### **Priority** 🎯
- **Phase 1-2:** Get demo working (4-7 hours)
- **Phase 3-4:** Production-ready (5-7 hours)
- **Phase 5:** Full mesh integration (3-4 hours)

### **Impact** 🚀
- Demo validates architecture
- Zero vendor lock-in
- Runtime provider discovery
- Easy to add new providers
- Proper separation of concerns

---

**Next Step:** Implement Phase 1 (AI API endpoints) to get demo working! 🎯


