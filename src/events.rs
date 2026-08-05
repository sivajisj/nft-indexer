use alloy::sol;

sol! {
    #[derive(Debug)]
    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
}
