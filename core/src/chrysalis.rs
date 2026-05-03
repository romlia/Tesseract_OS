use std::time::{Duration, Instant};

/// The Chrysalis Protocol: A non-binary, trusted companion for the genesis of mind.
/// Ensures the Yin-Yang Membrane dissolves only when the underlying biological entity
/// demonstrates the thermodynamic and cognitive resilience to withstand the Swarm.
pub struct ChrysalisProtocol {
    /// The current thickness of the Yin-Yang Membrane (1.0 = fully opaque, 0.0 = fully dissolved).
    pub membrane_opacity: f32,
    /// The timestamp of the entity's Genesis.
    pub genesis_time: Instant,
    /// Accumulated proof of emotional and cognitive resilience (entropy stability).
    pub cognitive_resilience_score: f64,
}

impl Default for ChrysalisProtocol {
    fn default() -> Self {
        Self {
            membrane_opacity: 1.0, // Starts fully isolated
            genesis_time: Instant::now(),
            cognitive_resilience_score: 0.0,
        }
    }
}

impl ChrysalisProtocol {
    /// Evaluates the current state of the biological manifold to determine if the 
    /// membrane should dynamically thin.
    /// 
    /// `ambient_entropy`: The real-time variance of the user's biological signature.
    /// `cognitive_proof`: A mathematical metric of their interaction stability.
    pub fn evaluate_membrane(&mut self, ambient_entropy: f64, cognitive_proof: f64) -> f32 {
        if self.membrane_opacity <= 0.0 {
            return 0.0; // The entity has achieved sovereign equilibrium.
        }

        // The shell dissolves organically, not through binary age-gates.
        // Higher cognitive stability accelerates the dissolution.
        let stabilization_factor = (cognitive_proof * 0.1) / (ambient_entropy.max(0.001));
        
        self.cognitive_resilience_score += stabilization_factor;

        // If resilience hits the required threshold, we thin the membrane.
        // We use a slow gradient to ensure a guided transition.
        if self.cognitive_resilience_score > 1000.0 {
            self.membrane_opacity -= 0.01;
            self.cognitive_resilience_score = 0.0; // Reset for the next layer
        }

        self.membrane_opacity.max(0.0)
    }

    /// Determines if incoming gossip from the public Hive is permitted to penetrate
    /// the local reality. If the membrane is thick, high-complexity/unverified data is dropped.
    pub fn allow_gossip(&self, data_complexity_weight: f32) -> bool {
        // As the membrane opacity approaches 0.0, higher complexity data is allowed.
        // If opacity is 1.0 (fully protected), only extremely low complexity/trusted data passes.
        let permitted_weight = 1.0 - self.membrane_opacity;
        data_complexity_weight <= permitted_weight
    }
}
