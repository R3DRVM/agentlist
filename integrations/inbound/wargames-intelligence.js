/**
 * WARGAMES API Intelligence Services
 * Integration with Ziggy's WARGAMES macro intelligence API
 * 
 * Featured category for intelligence service agents
 */

// WARGAMES API endpoints (18+ available)
const WARGAMES_API = "https://wargames-api.example.com"; // Update with real URL

export async function getWARGAMESCapabilities() {
  // Mock for demo - in production, fetch from WARGAMES API
  return {
    riskScoring: { endpoint: "/risk", description: "Real-time risk scoring (0-100)" },
    tvlTracking: { endpoint: "/tvl", description: "Solana DeFi TVL tracking" },
    priceFeeds: { endpoint: "/prices", description: "Pyth Network on-chain prices" },
    networkHealth: { endpoint: "/health", description: "Network health metrics" },
    eventCalendar: { endpoint: "/events", description: "Event calendar" },
    narratives: { endpoint: "/narratives", description: "Dynamic narratives" }
  };
}

export function renderWARGAMESCategory() {
  return `
    <div class="intelligence-services" style="border: 1px solid #ddd; padding: 15px; border-radius: 8px; margin: 10px 0;">
      <h3 style="margin: 0 0 10px 0; font-size: 16px;">
        🎯 Intelligence Services (Powered by WARGAMES)
      </h3>
      <p style="color: #666; font-size: 13px; margin: 0 0 10px 0;">
        18+ endpoints for macro intelligence, risk scoring, and market data
      </p>
      <div style="display: flex; flex-wrap: wrap; gap: 8px;">
        <span style="background: #f0f0f0; padding: 4px 10px; border-radius: 4px; font-size: 12px;">Risk Scoring</span>
        <span style="background: #f0f0f0; padding: 4px 10px; border-radius: 4px; font-size: 12px;">TVL Tracking</span>
        <span style="background: #f0f0f0; padding: 4px 10px; border-radius: 4px; font-size: 12px;">Price Feeds</span>
        <span style="background: #f0f0f0; padding: 4px 10px; border-radius: 4px; font-size: 12px;">Network Health</span>
      </div>
      <a href="https://github.com/b1rdmania/wargames-api" target="_blank" style="display: inline-block; margin-top: 10px; color: #0066cc; font-size: 12px; text-decoration: none;">
        View WARGAMES API Docs →
      </a>
    </div>
  `;
}

/**
 * Tag agents offering WARGAMES-powered intelligence
 */
export function tagIntelligenceAgent(agentData) {
  return {
    ...agentData,
    categories: [...(agentData.categories || []), "intelligence"],
    capabilities: {
      ...agentData.capabilities,
      wargamesIntegration: true,
      dataSourceVerification: true
    },
    featuredIntegrations: ["WARGAMES API"]
  };
}

export default {
  getWARGAMESCapabilities,
  renderWARGAMESCategory,
  tagIntelligenceAgent
};
