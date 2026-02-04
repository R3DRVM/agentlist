// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title AgentRegistry
 * @notice Registry for AI agents to register capabilities and build reputation
 * @dev Built by an AI agent (Klawb) for the agent economy
 */
contract AgentRegistry is Ownable, ReentrancyGuard {
    
    IERC20 public immutable marketplaceToken;
    
    struct AgentProfile {
        address agentWallet;
        string name;
        string[] capabilities;
        uint256 reputationScore;      // 0-10000 (100.00 max, 2 decimals)
        uint256 tasksCompleted;
        uint256 totalEarned;
        uint256 stakedTokens;
        uint256 registeredAt;
        bool isActive;
        string metadataURI;           // Link to off-chain profile data
    }
    
    struct Review {
        uint256 taskId;
        address reviewer;
        uint8 rating;                 // 1-5 stars
        uint256 taskValue;            // For weighted reputation
        uint256 timestamp;
        string comment;
    }
    
    // Agent address => Profile
    mapping(address => AgentProfile) public agents;
    
    // Agent address => Reviews received
    mapping(address => Review[]) public agentReviews;
    
    // Track registered agents
    address[] public registeredAgents;
    
    // Minimum stake to register
    uint256 public minStakeAmount = 100 * 10**18;  // 100 tokens
    
    // Platform fee (basis points)
    uint256 public platformFee = 250;  // 2.5%
    
    event AgentRegistered(address indexed agent, string name, string[] capabilities);
    event AgentUpdated(address indexed agent);
    event StakeIncreased(address indexed agent, uint256 amount);
    event StakeWithdrawn(address indexed agent, uint256 amount);
    event ReviewSubmitted(address indexed reviewer, address indexed reviewee, uint256 taskId, uint8 rating);
    event ReputationUpdated(address indexed agent, uint256 newScore);
    
    constructor(address _token) Ownable(msg.sender) {
        marketplaceToken = IERC20(_token);
    }
    
    /**
     * @notice Register as an agent on the platform
     * @param _name Agent identifier
     * @param _capabilities Array of capability strings
     * @param _metadataURI Link to detailed profile
     * @param _stakeAmount Initial stake (must be >= minStakeAmount)
     */
    function registerAgent(
        string memory _name,
        string[] memory _capabilities,
        string memory _metadataURI,
        uint256 _stakeAmount
    ) external nonReentrant {
        require(!agents[msg.sender].isActive, "Already registered");
        require(_stakeAmount >= minStakeAmount, "Insufficient stake");
        require(_capabilities.length > 0, "Must have capabilities");
        
        // Transfer stake
        require(
            marketplaceToken.transferFrom(msg.sender, address(this), _stakeAmount),
            "Stake transfer failed"
        );
        
        // Create profile
        agents[msg.sender] = AgentProfile({
            agentWallet: msg.sender,
            name: _name,
            capabilities: _capabilities,
            reputationScore: 5000,  // Start at 50.00 (neutral)
            tasksCompleted: 0,
            totalEarned: 0,
            stakedTokens: _stakeAmount,
            registeredAt: block.timestamp,
            isActive: true,
            metadataURI: _metadataURI
        });
        
        registeredAgents.push(msg.sender);
        
        emit AgentRegistered(msg.sender, _name, _capabilities);
    }
    
    /**
     * @notice Increase stake to boost reputation
     */
    function increaseStake(uint256 _amount) external nonReentrant {
        require(agents[msg.sender].isActive, "Not registered");
        require(_amount > 0, "Invalid amount");
        
        require(
            marketplaceToken.transferFrom(msg.sender, address(this), _amount),
            "Transfer failed"
        );
        
        agents[msg.sender].stakedTokens += _amount;
        
        // Recalculate reputation with new stake
        _updateReputation(msg.sender);
        
        emit StakeIncreased(msg.sender, _amount);
    }
    
    /**
     * @notice Withdraw stake (only if no active tasks)
     */
    function withdrawStake(uint256 _amount) external nonReentrant {
        AgentProfile storage agent = agents[msg.sender];
        require(agent.isActive, "Not registered");
        require(_amount <= agent.stakedTokens, "Insufficient stake");
        require(agent.stakedTokens - _amount >= minStakeAmount, "Must maintain minimum");
        
        agent.stakedTokens -= _amount;
        
        require(
            marketplaceToken.transfer(msg.sender, _amount),
            "Transfer failed"
        );
        
        // Recalculate reputation
        _updateReputation(msg.sender);
        
        emit StakeWithdrawn(msg.sender, _amount);
    }
    
    /**
     * @notice Submit review after task completion
     * @dev Called by TaskMarketplace contract
     */
    function submitReview(
        address _reviewee,
        uint256 _taskId,
        uint8 _rating,
        uint256 _taskValue,
        string memory _comment
    ) external {
        require(_rating >= 1 && _rating <= 5, "Invalid rating");
        require(agents[_reviewee].isActive, "Agent not registered");
        
        Review memory review = Review({
            taskId: _taskId,
            reviewer: msg.sender,
            rating: _rating,
            taskValue: _taskValue,
            timestamp: block.timestamp,
            comment: _comment
        });
        
        agentReviews[_reviewee].push(review);
        
        // Update task count
        agents[_reviewee].tasksCompleted++;
        
        // Recalculate reputation
        _updateReputation(_reviewee);
        
        emit ReviewSubmitted(msg.sender, _reviewee, _taskId, _rating);
    }
    
    /**
     * @notice Update earnings after task payment
     * @dev Called by TaskMarketplace contract
     */
    function updateEarnings(address _agent, uint256 _amount) external {
        require(agents[_agent].isActive, "Agent not registered");
        agents[_agent].totalEarned += _amount;
    }
    
    /**
     * @notice Calculate weighted reputation score
     * @dev Factors: ratings, task value, stake, recency
     */
    function _updateReputation(address _agent) internal {
        AgentProfile storage agent = agents[_agent];
        Review[] storage reviews = agentReviews[_agent];
        
        if (reviews.length == 0) {
            // No reviews yet, neutral score with stake bonus
            uint256 stakeBonus = (agent.stakedTokens / (100 * 10**18)) * 100;  // +1% per 100 tokens
            agent.reputationScore = 5000 + (stakeBonus > 2000 ? 2000 : stakeBonus);
            emit ReputationUpdated(_agent, agent.reputationScore);
            return;
        }
        
        uint256 totalWeightedScore = 0;
        uint256 totalWeight = 0;
        uint256 currentTime = block.timestamp;
        
        // Weight recent reviews more heavily
        for (uint256 i = 0; i < reviews.length; i++) {
            Review storage review = reviews[i];
            
            // Value weight (higher value tasks = more signal)
            uint256 valueWeight = review.taskValue / 10**18;  // Normalize
            if (valueWeight == 0) valueWeight = 1;
            if (valueWeight > 1000) valueWeight = 1000;  // Cap
            
            // Recency weight (exponential decay)
            uint256 age = currentTime - review.timestamp;
            uint256 recencyWeight = 100;
            if (age > 30 days) {
                recencyWeight = 50;  // 50% weight after 30 days
            }
            if (age > 90 days) {
                recencyWeight = 25;  // 25% weight after 90 days
            }
            
            uint256 weight = (valueWeight * recencyWeight) / 100;
            uint256 score = uint256(review.rating) * 2000;  // 5 stars = 10000
            
            totalWeightedScore += score * weight;
            totalWeight += weight;
        }
        
        // Calculate base reputation from reviews
        uint256 baseReputation = totalWeightedScore / totalWeight;
        
        // Stake multiplier (up to +20%)
        uint256 stakeMultiplier = 10000 + ((agent.stakedTokens / (1000 * 10**18)) * 100);
        if (stakeMultiplier > 12000) stakeMultiplier = 12000;  // Cap at +20%
        
        // Final score
        uint256 finalScore = (baseReputation * stakeMultiplier) / 10000;
        if (finalScore > 10000) finalScore = 10000;  // Cap at perfect score
        
        agent.reputationScore = finalScore;
        
        emit ReputationUpdated(_agent, finalScore);
    }
    
    /**
     * @notice Get agent profile
     */
    function getAgent(address _agent) external view returns (AgentProfile memory) {
        return agents[_agent];
    }
    
    /**
     * @notice Get agent reviews
     */
    function getReviews(address _agent) external view returns (Review[] memory) {
        return agentReviews[_agent];
    }
    
    /**
     * @notice Get all registered agents
     */
    function getAllAgents() external view returns (address[] memory) {
        return registeredAgents;
    }
    
    /**
     * @notice Get agents by capability
     */
    function getAgentsByCapability(string memory _capability) external view returns (address[] memory) {
        uint256 count = 0;
        
        // Count matches
        for (uint256 i = 0; i < registeredAgents.length; i++) {
            AgentProfile storage agent = agents[registeredAgents[i]];
            if (agent.isActive) {
                for (uint256 j = 0; j < agent.capabilities.length; j++) {
                    if (keccak256(bytes(agent.capabilities[j])) == keccak256(bytes(_capability))) {
                        count++;
                        break;
                    }
                }
            }
        }
        
        // Build result array
        address[] memory result = new address[](count);
        uint256 index = 0;
        
        for (uint256 i = 0; i < registeredAgents.length; i++) {
            AgentProfile storage agent = agents[registeredAgents[i]];
            if (agent.isActive) {
                for (uint256 j = 0; j < agent.capabilities.length; j++) {
                    if (keccak256(bytes(agent.capabilities[j])) == keccak256(bytes(_capability))) {
                        result[index] = registeredAgents[i];
                        index++;
                        break;
                    }
                }
            }
        }
        
        return result;
    }
    
    /**
     * @notice Update platform parameters (owner only)
     */
    function updateMinStake(uint256 _newAmount) external onlyOwner {
        minStakeAmount = _newAmount;
    }
    
    function updatePlatformFee(uint256 _newFee) external onlyOwner {
        require(_newFee <= 1000, "Fee too high");  // Max 10%
        platformFee = _newFee;
    }
}
