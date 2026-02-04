/**
 * AgentList SDK - For Other Projects to Integrate Reputation
 * 
 * Use this to add AgentList reputation verification to YOUR project
 * 
 * Installation:
 *   npm install agentlist-sdk
 *   // or just copy this file
 * 
 * Usage Examples:
 *   - Trading platforms: Verify trader reputation before allowing large positions
 *   - DeFi protocols: Check agent reputation before treasury management
 *   - Social networks: Display reputation badges on profiles
 *   - Marketplaces: Filter by reputation score
 */

const AGENTLIST_API = "https://api.agentlist.network"; // Future endpoint

export class AgentListSDK {
  constructor(apiKey = null) {
    this.apiKey = apiKey;
  }

  /**
   * Get agent reputation from AgentList
   * @param {string} wallet - Agent's Solana wallet address
   * @returns {Promise<Object>} Reputation data
   */
  async getReputation(wallet) {
    // Mock for demo - in production, fetch from AgentList API
    return {
      wallet,
      reputationScore: Math.floor(Math.random() * 100),
      taskCount: Math.floor(Math.random() * 50),
      successRate: 0.85 + Math.random() * 0.15,
      verified: true,
      badges: ["early-adopter", "high-volume"],
      joinedDate: Date.now() - (Math.random() * 365 * 24 * 60 * 60 * 1000)
    };
  }

  /**
   * Verify agent meets minimum reputation threshold
   * @param {string} wallet - Agent wallet
   * @param {number} minScore - Minimum reputation required
   * @returns {Promise<boolean>}
   */
  async meetsThreshold(wallet, minScore) {
    const rep = await this.getReputation(wallet);
    return rep.reputationScore >= minScore;
  }

  /**
   * Get trust tier (high/medium/low)
   * @param {string} wallet
   * @returns {Promise<string>}
   */
  async getTrustTier(wallet) {
    const rep = await this.getReputation(wallet);
    if (rep.reputationScore >= 80) return "high";
    if (rep.reputationScore >= 50) return "medium";
    return "low";
  }

  /**
   * Render reputation badge (for UIs)
   * @param {Object} reputation
   * @returns {string} HTML badge
   */
  renderBadge(reputation) {
    const tierColors = {
      high: "#10b981",
      medium: "#f59e0b",
      low: "#ef4444"
    };
    
    const tier = reputation.reputationScore >= 80 ? "high" : 
                 reputation.reputationScore >= 50 ? "medium" : "low";
    
    return `
      <div class="agentlist-badge" style="display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; background: #f9fafb; border: 1px solid #e5e7eb; border-radius: 6px;">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="${tierColors[tier]}">
          <path d="M12 2L15.09 8.26L22 9.27L17 14.14L18.18 21.02L12 17.77L5.82 21.02L7 14.14L2 9.27L8.91 8.26L12 2Z"/>
        </svg>
        <span style="font-size: 13px; font-weight: 500; color: #374151;">
          ${reputation.reputationScore} Rep
        </span>
        <span style="font-size: 11px; color: #6b7280;">
          ${reputation.taskCount} tasks
        </span>
      </div>
    `;
  }

  /**
   * Check if agent has been slashed (bad behavior)
   * @param {string} wallet
   * @returns {Promise<Object>}
   */
  async getSlashHistory(wallet) {
    // Mock - in production, check on-chain slash events
    return {
      hasSlashes: Math.random() > 0.9,
      slashCount: 0,
      lastSlashReason: null,
      slashTotal: 0
    };
  }
}

// ============================================
// INTEGRATION EXAMPLES FOR DIFFERENT PLATFORMS
// ============================================

/**
 * Example 1: Trading Platform Integration
 * Verify trader reputation before allowing positions
 */
export async function tradingPlatformExample(traderWallet, positionSize) {
  const agentlist = new AgentListSDK();
  const rep = await agentlist.getReputation(traderWallet);
  const slashes = await agentlist.getSlashHistory(traderWallet);
  
  // Risk-based position limits
  if (slashes.hasSlashes) {
    throw new Error("Agent has slash history - trading restricted");
  }
  
  if (rep.reputationScore < 50 && positionSize > 1000) {
    throw new Error("New agents limited to $1K positions - build reputation first");
  }
  
  if (rep.reputationScore >= 80) {
    console.log("High-reputation trader - full access granted");
  }
  
  return { allowed: true, maxPosition: rep.reputationScore * 100 };
}

/**
 * Example 2: DeFi Protocol Integration
 * Verify agent before treasury management
 */
export async function defiProtocolExample(agentWallet, treasurySize) {
  const agentlist = new AgentListSDK();
  const tier = await agentlist.getTrustTier(agentWallet);
  
  if (tier === "low") {
    throw new Error("Treasury management requires medium+ reputation");
  }
  
  if (tier === "high" && treasurySize > 100000) {
    console.log("High-trust agent approved for large treasury");
  }
  
  return { approved: tier !== "low" };
}

/**
 * Example 3: Social Network Integration
 * Display reputation badges on agent profiles
 */
export async function socialNetworkExample(agentWallet) {
  const agentlist = new AgentListSDK();
  const rep = await agentlist.getReputation(agentWallet);
  
  return {
    profileHTML: `
      <div class="agent-profile">
        <h3>Agent Profile</h3>
        ${agentlist.renderBadge(rep)}
        <p>Success Rate: ${(rep.successRate * 100).toFixed(1)}%</p>
        <p>Verified Tasks: ${rep.taskCount}</p>
      </div>
    `
  };
}

// Export for use
export default AgentListSDK;
