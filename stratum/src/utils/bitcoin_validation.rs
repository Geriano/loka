// Bitcoin address validation utilities
//
// Provides comprehensive validation for Bitcoin addresses including:
// - Legacy addresses (P2PKH, P2SH)
// - SegWit addresses (P2WPKH, P2WSH)
// - Taproot addresses (P2TR)
// - Worker name format validation (address.worker_name)

use tracing::debug;

/// Bitcoin address validation result
#[derive(Debug, Clone, PartialEq)]
pub enum AddressValidationResult {
    /// Valid Bitcoin address
    Valid {
        address: String,
        address_type: BitcoinAddressType,
    },
    /// Valid worker name format (bitcoin_address.worker_name)
    ValidWorkerFormat {
        bitcoin_address: String,
        worker_name: String,
        address_type: BitcoinAddressType,
    },
    /// Invalid address format
    Invalid { reason: String },
}

/// Bitcoin address types
#[derive(Debug, Clone, PartialEq)]
pub enum BitcoinAddressType {
    /// Legacy Pay-to-Public-Key-Hash (P2PKH) - starts with '1'
    P2PKH,
    /// Legacy Pay-to-Script-Hash (P2SH) - starts with '3'  
    P2SH,
    /// SegWit Pay-to-Witness-Public-Key-Hash (P2WPKH) - starts with 'bc1q'
    P2WPKH,
    /// SegWit Pay-to-Witness-Script-Hash (P2WSH) - starts with 'bc1q' (64 chars)
    P2WSH,
    /// Taproot Pay-to-Taproot (P2TR) - starts with 'bc1p'
    P2TR,
    /// Testnet addresses - starts with 'm', 'n', '2', or 'tb1'
    Testnet,
}

/// Bitcoin address validator
pub struct BitcoinAddressValidator {
    allow_testnet: bool,
}

impl BitcoinAddressValidator {
    /// Create new validator
    pub fn new(allow_testnet: bool) -> Self {
        Self { allow_testnet }
    }

    /// Validate a Bitcoin address or worker name format
    pub fn validate_address(&self, input: &str) -> AddressValidationResult {
        if input.is_empty() {
            return AddressValidationResult::Invalid {
                reason: "Address cannot be empty".to_string(),
            };
        }

        // Check if this is a worker name format (address.worker_name)
        if let Some((address_part, worker_part)) = input.split_once('.') {
            // Validate the Bitcoin address part
            match self.validate_bitcoin_address(address_part) {
                AddressValidationResult::Valid { address_type, .. } => {
                    // Validate worker name part
                    if self.is_valid_worker_name(worker_part) {
                        return AddressValidationResult::ValidWorkerFormat {
                            bitcoin_address: address_part.to_string(),
                            worker_name: worker_part.to_string(),
                            address_type,
                        };
                    } else {
                        return AddressValidationResult::Invalid {
                            reason: format!("Invalid worker name: {worker_part}"),
                        };
                    }
                }
                AddressValidationResult::Invalid { reason } => {
                    return AddressValidationResult::Invalid {
                        reason: format!("Invalid Bitcoin address in worker format: {reason}"),
                    };
                }
                _ => unreachable!(),
            }
        }

        // Validate as standalone Bitcoin address
        self.validate_bitcoin_address(input)
    }

    /// Validate a Bitcoin address (without worker name)
    fn validate_bitcoin_address(&self, address: &str) -> AddressValidationResult {
        // Check basic length constraints
        if address.len() < 26 || address.len() > 62 {
            return AddressValidationResult::Invalid {
                reason: format!("Invalid address length: {}", address.len()),
            };
        }

        // Determine address type and validate
        let address_type = match self.detect_address_type(address) {
            Some(addr_type) => addr_type,
            None => {
                return AddressValidationResult::Invalid {
                    reason: "Unrecognized address format".to_string(),
                };
            }
        };

        // Check if testnet addresses are allowed
        if matches!(address_type, BitcoinAddressType::Testnet) && !self.allow_testnet {
            return AddressValidationResult::Invalid {
                reason: "Testnet addresses not allowed".to_string(),
            };
        }

        // Perform format-specific validation
        match self.validate_address_format(address, &address_type) {
            Ok(()) => AddressValidationResult::Valid {
                address: address.to_string(),
                address_type,
            },
            Err(reason) => AddressValidationResult::Invalid { reason },
        }
    }

    /// Detect Bitcoin address type based on prefix
    fn detect_address_type(&self, address: &str) -> Option<BitcoinAddressType> {
        if address.starts_with('1') {
            Some(BitcoinAddressType::P2PKH)
        } else if address.starts_with('3') {
            Some(BitcoinAddressType::P2SH)
        } else if address.starts_with("bc1q") {
            // Distinguish between P2WPKH (42 chars) and P2WSH (62 chars)
            if address.len() == 42 {
                Some(BitcoinAddressType::P2WPKH)
            } else if address.len() == 62 {
                Some(BitcoinAddressType::P2WSH)
            } else {
                None
            }
        } else if address.starts_with("bc1p") {
            Some(BitcoinAddressType::P2TR)
        } else if address.starts_with('m') || address.starts_with('n') || address.starts_with('2') || address.starts_with("tb1") {
            Some(BitcoinAddressType::Testnet)
        } else {
            None
        }
    }

    /// Validate address format based on type
    fn validate_address_format(
        &self,
        address: &str,
        address_type: &BitcoinAddressType,
    ) -> Result<(), String> {
        match address_type {
            BitcoinAddressType::P2PKH | BitcoinAddressType::P2SH => {
                self.validate_base58_address(address)
            }
            BitcoinAddressType::P2WPKH | BitcoinAddressType::P2WSH | BitcoinAddressType::P2TR => {
                self.validate_bech32_address(address)
            }
            BitcoinAddressType::Testnet => {
                if address.starts_with("tb1") {
                    self.validate_bech32_address(address)
                } else {
                    self.validate_base58_address(address)
                }
            }
        }
    }

    /// Validate Base58 encoded address (P2PKH, P2SH, Testnet legacy)
    fn validate_base58_address(&self, address: &str) -> Result<(), String> {
        // Check character set (Base58: no 0, O, I, l)
        const BASE58_CHARS: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

        for ch in address.chars() {
            if !BASE58_CHARS.contains(ch) {
                return Err(format!("Invalid Base58 character: {ch}"));
            }
        }

        // Length validation (25 bytes when decoded, 34-35 chars encoded)
        if address.len() < 26 || address.len() > 35 {
            return Err("Invalid Base58 address length".to_string());
        }

        // Note: In production, you'd want to perform actual Base58 decoding and checksum validation
        // For now, we do basic format validation to avoid additional dependencies
        debug!("Base58 address validation passed for: {}", address);
        Ok(())
    }

    /// Validate Bech32 encoded address (SegWit, Taproot)
    fn validate_bech32_address(&self, address: &str) -> Result<(), String> {
        // Check character set (Bech32: lowercase letters and numbers, no 1, b, i, o)
        const BECH32_CHARS: &str = "023456789acdefghjklmnpqrstuvwxyz";

        // Split into HRP (Human Readable Part) and data
        if let Some(separator_pos) = address.rfind('1') {
            let hrp = &address[..separator_pos];
            let data = &address[separator_pos + 1..];

            // Validate HRP
            if hrp != "bc" && hrp != "tb" {
                return Err(format!("Invalid Bech32 HRP: {hrp}"));
            }

            // Validate data part characters
            for ch in data.chars() {
                if !BECH32_CHARS.contains(ch) {
                    return Err(format!("Invalid Bech32 character: {ch}"));
                }
            }

            // Length validation for data part
            if data.len() < 6 {
                return Err("Bech32 data too short".to_string());
            }

            // Note: In production, you'd want to perform actual Bech32 decoding and checksum validation
            debug!("Bech32 address validation passed for: {}", address);
            Ok(())
        } else {
            Err("Missing Bech32 separator '1'".to_string())
        }
    }

    /// Validate worker name format
    fn is_valid_worker_name(&self, worker_name: &str) -> bool {
        // Worker name should be alphanumeric with underscores/hyphens, 1-64 chars
        if worker_name.is_empty() || worker_name.len() > 64 {
            return false;
        }

        worker_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Get a user-friendly description of the validation result
    pub fn describe_result(&self, result: &AddressValidationResult) -> String {
        match result {
            AddressValidationResult::Valid { address_type, .. } => {
                format!("Valid {} address", self.describe_address_type(address_type))
            }
            AddressValidationResult::ValidWorkerFormat { address_type, .. } => {
                format!(
                    "Valid worker format with {} address",
                    self.describe_address_type(address_type)
                )
            }
            AddressValidationResult::Invalid { reason } => {
                format!("Invalid: {reason}")
            }
        }
    }

    /// Get human-readable description of address type
    fn describe_address_type(&self, address_type: &BitcoinAddressType) -> &'static str {
        match address_type {
            BitcoinAddressType::P2PKH => "Legacy P2PKH",
            BitcoinAddressType::P2SH => "Legacy P2SH",
            BitcoinAddressType::P2WPKH => "SegWit P2WPKH",
            BitcoinAddressType::P2WSH => "SegWit P2WSH",
            BitcoinAddressType::P2TR => "Taproot P2TR",
            BitcoinAddressType::Testnet => "Testnet",
        }
    }
}

impl Default for BitcoinAddressValidator {
    fn default() -> Self {
        Self::new(false) // Production mode - no testnet by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_legacy_addresses() {
        let validator = BitcoinAddressValidator::new(false);

        // Valid P2PKH address
        let result = validator.validate_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa");
        assert!(matches!(
            result,
            AddressValidationResult::Valid {
                address_type: BitcoinAddressType::P2PKH,
                ..
            }
        ));

        // Valid P2SH address
        let result = validator.validate_address("3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy");
        assert!(matches!(
            result,
            AddressValidationResult::Valid {
                address_type: BitcoinAddressType::P2SH,
                ..
            }
        ));
    }

    #[test]
    fn test_valid_segwit_addresses() {
        let validator = BitcoinAddressValidator::new(false);

        // Valid P2WPKH address (42 chars)
        let result = validator.validate_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
        assert!(matches!(
            result,
            AddressValidationResult::Valid {
                address_type: BitcoinAddressType::P2WPKH,
                ..
            }
        ));

        // Valid P2TR address
        let result = validator
            .validate_address("bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr");
        assert!(matches!(
            result,
            AddressValidationResult::Valid {
                address_type: BitcoinAddressType::P2TR,
                ..
            }
        ));
    }

    #[test]
    fn test_valid_worker_format() {
        let validator = BitcoinAddressValidator::new(false);

        // Valid worker format with P2PKH address
        let result = validator.validate_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa.worker1");
        assert!(matches!(
            result,
            AddressValidationResult::ValidWorkerFormat { .. }
        ));

        // Valid worker format with SegWit address
        let result =
            validator.validate_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4.miner_01");
        assert!(matches!(
            result,
            AddressValidationResult::ValidWorkerFormat { .. }
        ));
    }

    #[test]
    fn test_invalid_addresses() {
        let validator = BitcoinAddressValidator::new(false);

        // Invalid test username
        let result = validator.validate_address("john.test");
        assert!(matches!(result, AddressValidationResult::Invalid { .. }));

        // Empty address
        let result = validator.validate_address("");
        assert!(matches!(result, AddressValidationResult::Invalid { .. }));

        // Invalid characters
        let result = validator.validate_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa0");
        assert!(matches!(result, AddressValidationResult::Invalid { .. }));
    }

    #[test]
    fn test_testnet_addresses() {
        // With testnet disabled
        let validator = BitcoinAddressValidator::new(false);
        let result = validator.validate_address("n2ZNV88uQbede7C5M5jzi6SyC3tjMrbhLQ");
        assert!(matches!(result, AddressValidationResult::Invalid { .. }));

        // With testnet enabled
        let validator_testnet = BitcoinAddressValidator::new(true);
        let result = validator_testnet.validate_address("n2ZNV88uQbede7C5M5jzi6SyC3tjMrbhLQ");
        assert!(matches!(
            result,
            AddressValidationResult::Valid {
                address_type: BitcoinAddressType::Testnet,
                ..
            }
        ));
    }
}
