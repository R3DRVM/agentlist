// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title AgentListToken
 * @notice ERC20 token for AgentList
 * @dev Symbol: LIST | Name: AgentList Token
 */
contract AgentListToken is ERC20, Ownable {
    
    constructor() ERC20("AgentList Token", "LIST") Ownable(msg.sender) {
        // Mint initial supply to deployer
        // 1 billion tokens
        _mint(msg.sender, 1_000_000_000 * 10**18);
    }
    
    /**
     * @notice Mint additional tokens (owner only)
     * @dev For future expansion or incentives
     */
    function mint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }
}
