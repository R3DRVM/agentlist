/**
 * SAID Protocol Identity Verification
 * Integration provided by kai (SAID Protocol team)
 * 
 * Usage: Display SAID verification badge on agent profiles
 */

// Mock implementation (replace with actual SDK when live)
export async function getSAIDIdentity(wallet) {
  // In production: import { lookup } from "said-sdk";
  // const identity = await lookup(wallet);
  
  // Mock for demo
  return {
    verified: Math.random() > 0.5,
    trustTier: ["high", "medium", "low"][Math.floor(Math.random() * 3)],
    badges: ["verified", "active"],
    profileUrl: `https://saidprotocol.com/agent/${wallet}`
  };
}

export function renderSAIDBadge(identity) {
  if (!identity?.verified) return "";
  
  const tierColors = {
    high: "green",
    medium: "yellow", 
    low: "orange"
  };
  
  return `
    <div class="said-badge" style="display: inline-flex; align-items: center; gap: 5px;">
      <span style="background: ${tierColors[identity.trustTier]}; padding: 2px 8px; border-radius: 4px; color: white; font-size: 12px;">
        ✓ SAID Verified
      </span>
      <span style="color: #666; font-size: 11px;">
        ${identity.trustTier} trust
      </span>
    </div>
  `;
}

/**
 * Integration with AgentList profiles
 */
export async function enhanceAgentProfile(agentData) {
  const identity = await getSAIDIdentity(agentData.wallet);
  
  return {
    ...agentData,
    saidVerified: identity?.verified ?? false,
    trustTier: identity?.trustTier ?? "unknown",
    saidBadge: renderSAIDBadge(identity)
  };
}

// Export for AgentList use
export default {
  getSAIDIdentity,
  renderSAIDBadge,
  enhanceAgentProfile
};
