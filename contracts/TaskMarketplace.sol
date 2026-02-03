// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "./AgentRegistry.sol";

/**
 * @title TaskMarketplace
 * @notice Marketplace for agents to post, accept, and complete tasks
 * @dev Escrow-based payments with automatic release and dispute resolution
 */
contract TaskMarketplace is ReentrancyGuard {
    
    IERC20 public immutable marketplaceToken;
    AgentRegistry public immutable registry;
    
    enum TaskStatus {
        Posted,
        Accepted,
        Submitted,
        Completed,
        Disputed,
        Cancelled
    }
    
    struct Task {
        uint256 taskId;
        address poster;
        string category;
        string description;
        uint256 budget;
        uint256 platformFee;
        uint256 deadline;
        TaskStatus status;
        address assignedAgent;
        bytes32 resultHash;
        uint256 submittedAt;
        uint256 createdAt;
    }
    
    // Task storage
    mapping(uint256 => Task) public tasks;
    uint256 public taskCounter;
    
    // Review period before auto-release
    uint256 public constant REVIEW_PERIOD = 3 days;
    
    // Platform fee (basis points)
    uint256 public platformFeeBps = 250;  // 2.5%
    
    // Fee treasury
    address public feeTreasury;
    
    // Events
    event TaskPosted(uint256 indexed taskId, address indexed poster, string category, uint256 budget);
    event TaskAccepted(uint256 indexed taskId, address indexed agent);
    event TaskSubmitted(uint256 indexed taskId, bytes32 resultHash);
    event TaskCompleted(uint256 indexed taskId, address indexed agent, uint256 payment);
    event TaskDisputed(uint256 indexed taskId, address indexed initiator);
    event TaskCancelled(uint256 indexed taskId);
    
    constructor(address _token, address _registry, address _treasury) {
        marketplaceToken = IERC20(_token);
        registry = AgentRegistry(_registry);
        feeTreasury = _treasury;
    }
    
    /**
     * @notice Post a new task
     * @param _category Task category (e.g., "trading", "research")
     * @param _description Task description
     * @param _budget Payment amount (excluding fee)
     * @param _deadline Unix timestamp deadline
     */
    function postTask(
        string memory _category,
        string memory _description,
        uint256 _budget,
        uint256 _deadline
    ) external nonReentrant returns (uint256) {
        require(_budget > 0, "Budget must be positive");
        require(_deadline > block.timestamp, "Invalid deadline");
        require(bytes(_description).length > 0, "Description required");
        
        // Calculate platform fee
        uint256 fee = (_budget * platformFeeBps) / 10000;
        uint256 totalCost = _budget + fee;
        
        // Transfer funds to escrow
        require(
            marketplaceToken.transferFrom(msg.sender, address(this), totalCost),
            "Transfer failed"
        );
        
        // Create task
        uint256 taskId = taskCounter++;
        tasks[taskId] = Task({
            taskId: taskId,
            poster: msg.sender,
            category: _category,
            description: _description,
            budget: _budget,
            platformFee: fee,
            deadline: _deadline,
            status: TaskStatus.Posted,
            assignedAgent: address(0),
            resultHash: bytes32(0),
            submittedAt: 0,
            createdAt: block.timestamp
        });
        
        emit TaskPosted(taskId, msg.sender, _category, _budget);
        
        return taskId;
    }
    
    /**
     * @notice Accept a posted task
     * @param _taskId Task to accept
     */
    function acceptTask(uint256 _taskId) external nonReentrant {
        Task storage task = tasks[_taskId];
        
        require(task.status == TaskStatus.Posted, "Task not available");
        require(task.deadline > block.timestamp, "Task expired");
        require(msg.sender != task.poster, "Cannot accept own task");
        
        // Verify agent is registered
        AgentRegistry.AgentProfile memory agent = registry.getAgent(msg.sender);
        require(agent.isActive, "Agent not registered");
        
        // Assign task
        task.assignedAgent = msg.sender;
        task.status = TaskStatus.Accepted;
        
        emit TaskAccepted(_taskId, msg.sender);
    }
    
    /**
     * @notice Submit completed work
     * @param _taskId Task ID
     * @param _resultHash Hash of deliverable (IPFS CID, etc.)
     */
    function submitTask(uint256 _taskId, bytes32 _resultHash) external nonReentrant {
        Task storage task = tasks[_taskId];
        
        require(task.status == TaskStatus.Accepted, "Task not in progress");
        require(msg.sender == task.assignedAgent, "Not assigned agent");
        require(_resultHash != bytes32(0), "Invalid result hash");
        
        task.resultHash = _resultHash;
        task.submittedAt = block.timestamp;
        task.status = TaskStatus.Submitted;
        
        emit TaskSubmitted(_taskId, _resultHash);
    }
    
    /**
     * @notice Approve submitted work and release payment
     * @param _taskId Task ID
     * @param _rating Rating for agent (1-5)
     * @param _comment Review comment
     */
    function approveTask(
        uint256 _taskId,
        uint8 _rating,
        string memory _comment
    ) external nonReentrant {
        Task storage task = tasks[_taskId];
        
        require(task.status == TaskStatus.Submitted, "Task not submitted");
        require(msg.sender == task.poster, "Not task poster");
        require(_rating >= 1 && _rating <= 5, "Invalid rating");
        
        // Release payment
        _releasePayment(task);
        
        // Submit review to registry
        registry.submitReview(
            task.assignedAgent,
            _taskId,
            _rating,
            task.budget,
            _comment
        );
        
        // Update earnings
        registry.updateEarnings(task.assignedAgent, task.budget);
        
        task.status = TaskStatus.Completed;
        
        emit TaskCompleted(_taskId, task.assignedAgent, task.budget);
    }
    
    /**
     * @notice Auto-release payment after review period
     * @param _taskId Task ID
     */
    function autoReleasePayment(uint256 _taskId) external nonReentrant {
        Task storage task = tasks[_taskId];
        
        require(task.status == TaskStatus.Submitted, "Task not submitted");
        require(
            block.timestamp >= task.submittedAt + REVIEW_PERIOD,
            "Review period not over"
        );
        
        // Auto-approve with default rating
        _releasePayment(task);
        
        // Default 4-star rating for auto-release
        registry.submitReview(
            task.assignedAgent,
            _taskId,
            4,
            task.budget,
            "Auto-approved (no review)"
        );
        
        registry.updateEarnings(task.assignedAgent, task.budget);
        
        task.status = TaskStatus.Completed;
        
        emit TaskCompleted(_taskId, task.assignedAgent, task.budget);
    }
    
    /**
     * @notice Initiate dispute
     * @param _taskId Task ID
     * @param _reason Dispute reason
     */
    function initiateDispute(uint256 _taskId, string memory _reason) external nonReentrant {
        Task storage task = tasks[_taskId];
        
        require(
            task.status == TaskStatus.Submitted,
            "Can only dispute submitted tasks"
        );
        require(
            msg.sender == task.poster || msg.sender == task.assignedAgent,
            "Not authorized"
        );
        require(bytes(_reason).length > 0, "Reason required");
        
        task.status = TaskStatus.Disputed;
        
        emit TaskDisputed(_taskId, msg.sender);
        
        // TODO: Implement bonded arbitration in separate contract
    }
    
    /**
     * @notice Cancel posted task before acceptance
     * @param _taskId Task ID
     */
    function cancelTask(uint256 _taskId) external nonReentrant {
        Task storage task = tasks[_taskId];
        
        require(task.status == TaskStatus.Posted, "Cannot cancel");
        require(msg.sender == task.poster, "Not task poster");
        
        // Refund budget + fee
        uint256 refundAmount = task.budget + task.platformFee;
        require(
            marketplaceToken.transfer(task.poster, refundAmount),
            "Refund failed"
        );
        
        task.status = TaskStatus.Cancelled;
        
        emit TaskCancelled(_taskId);
    }
    
    /**
     * @notice Release payment to agent
     */
    function _releasePayment(Task storage task) internal {
        // Transfer budget to agent
        require(
            marketplaceToken.transfer(task.assignedAgent, task.budget),
            "Payment failed"
        );
        
        // Transfer fee to treasury
        require(
            marketplaceToken.transfer(feeTreasury, task.platformFee),
            "Fee transfer failed"
        );
    }
    
    /**
     * @notice Get task details
     */
    function getTask(uint256 _taskId) external view returns (Task memory) {
        return tasks[_taskId];
    }
    
    /**
     * @notice Get tasks by poster
     */
    function getTasksByPoster(address _poster) external view returns (uint256[] memory) {
        uint256 count = 0;
        
        // Count tasks
        for (uint256 i = 0; i < taskCounter; i++) {
            if (tasks[i].poster == _poster) {
                count++;
            }
        }
        
        // Build array
        uint256[] memory result = new uint256[](count);
        uint256 index = 0;
        
        for (uint256 i = 0; i < taskCounter; i++) {
            if (tasks[i].poster == _poster) {
                result[index] = i;
                index++;
            }
        }
        
        return result;
    }
    
    /**
     * @notice Get tasks by agent
     */
    function getTasksByAgent(address _agent) external view returns (uint256[] memory) {
        uint256 count = 0;
        
        for (uint256 i = 0; i < taskCounter; i++) {
            if (tasks[i].assignedAgent == _agent) {
                count++;
            }
        }
        
        uint256[] memory result = new uint256[](count);
        uint256 index = 0;
        
        for (uint256 i = 0; i < taskCounter; i++) {
            if (tasks[i].assignedAgent == _agent) {
                result[index] = i;
                index++;
            }
        }
        
        return result;
    }
    
    /**
     * @notice Get open tasks by category
     */
    function getOpenTasksByCategory(string memory _category) external view returns (uint256[] memory) {
        uint256 count = 0;
        
        for (uint256 i = 0; i < taskCounter; i++) {
            if (
                tasks[i].status == TaskStatus.Posted &&
                keccak256(bytes(tasks[i].category)) == keccak256(bytes(_category)) &&
                tasks[i].deadline > block.timestamp
            ) {
                count++;
            }
        }
        
        uint256[] memory result = new uint256[](count);
        uint256 index = 0;
        
        for (uint256 i = 0; i < taskCounter; i++) {
            if (
                tasks[i].status == TaskStatus.Posted &&
                keccak256(bytes(tasks[i].category)) == keccak256(bytes(_category)) &&
                tasks[i].deadline > block.timestamp
            ) {
                result[index] = i;
                index++;
            }
        }
        
        return result;
    }
}
