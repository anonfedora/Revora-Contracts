import re
import os

path = '/home/edohwares/Desktop/Room/grantfox/Revora-Contracts/src/test_auth.rs'

with open(path, 'r') as f:
    content = f.read()

# Fix common variable definitions
content = content.replace('&&issuer', '&issuer')

# Fix setup_offering usage: rename _issuer to issuer if needed
content = content.replace('let (_issuer, token) = setup_offering', 'let (issuer, token) = setup_offering')

# report_revenue: (issuer, namespace, token, asset, amount, period, override)
content = re.sub(r'\.(try_)?report_revenue\(&?([^,]+),\s*&?token,\s*&?token,\s*&?([0-9]+),\s*&?([0-9a-z_]+),\s*&?false\)',
                 r'.\1report_revenue(&\2, &symbol_short!("def"), &token, &token, &\3, &\4, &false)', content)

# deposit_revenue: (issuer, namespace, token, payment_token, amount, period)
content = re.sub(r'\.(try_)?deposit_revenue\(&?([^,]+),\s*&?token,\s*&?payment_token,\s*&?([0-9]+),\s*&?([0-9a-z_]+)\)',
                 r'.\1deposit_revenue(&\2, &symbol_short!("def"), &token, &payment_token, &\3, &\4)', content)

# set_holder_share: (issuer, namespace, token, holder, bps)
content = re.sub(r'\.(try_)?set_holder_share\(&?([^,]+),\s*&?token,\s*&?holder,\s*&?([0-9u_]+)\)',
                 r'.\1set_holder_share(&\2, &symbol_short!("def"), &token, &holder, &\3)', content)

# set_concentration_limit: (issuer, namespace, token, limit, enforceable)
content = re.sub(r'\.(try_)?set_concentration_limit\(&?([^,]+),\s*&?token,\s*&?([0-9u_]+),\s*&?true\)',
                 r'.\1set_concentration_limit(&\2, &symbol_short!("def"), &token, &\3, &true)', content)

# set_rounding_mode: (issuer, namespace, token, mode)
content = re.sub(r'\.(try_)?set_rounding_mode\(&?([^,]+),\s*&?token,\s*&?([^)]+)\)',
                 r'.\1set_rounding_mode(&\2, &symbol_short!("def"), &token, \3)', content)

# set_min_revenue_threshold: (issuer, namespace, token, threshold)
content = re.sub(r'\.(try_)?set_min_revenue_threshold\(&?([^,]+),\s*&?token,\s*&?([0-9ia-z_]+)\)',
                 r'.\1set_min_revenue_threshold(&\2, &symbol_short!("def"), &token, &\3)', content)

# set_claim_delay: (issuer, namespace, token, delay)
content = re.sub(r'\.(try_)?set_claim_delay\(&?([^,]+),\s*&?token,\s*&?([0-9a-z_]+)\)',
                 r'.\1set_claim_delay(&\2, &symbol_short!("def"), &token, &\3)', content)

# set_offering_metadata: (issuer, namespace, token, meta)
content = re.sub(r'\.(try_)?set_offering_metadata\(&?([^,]+),\s*&?token,\s*&?([^)]+)\)',
                 r'.\1set_offering_metadata(&\2, &symbol_short!("def"), &token, \3)', content)

# blacklist_add: (caller, issuer, namespace, token, investor)
content = re.sub(r'\.(try_)?blacklist_add\(&?([^,]+),\s*&?token,\s*&?investor\)',
                 r'.\1blacklist_add(&\2, &issuer, &symbol_short!("def"), &token, &investor)', content)

# blacklist_remove: (caller, issuer, namespace, token, investor)
content = re.sub(r'\.(try_)?blacklist_remove\(&?([^,]+),\s*&?token,\s*&?investor\)',
                 r'.\1blacklist_remove(&\2, &issuer, &symbol_short!("def"), &token, &investor)', content)

# get_holder_share, get_claim_delay, etc.
content = re.sub(r'\.get_holder_share\(&token,\s*&holder\)', r'.get_holder_share(&issuer, &symbol_short!("def"), &token, &holder)', content)
content = re.sub(r'\.get_claim_delay\(&token\)', r'.get_claim_delay(&issuer, &symbol_short!("def"), &token)', content)
content = re.sub(r'\.get_blacklist\(&token\)', r'.get_blacklist(&issuer, &symbol_short!("def"), &token)', content)

# Fix mismatched types in is_blacklisted (Address vs &Address)
content = re.sub(r'is_blacklisted\(&?&?issuer,\s*&symbol_short!\("def"\),\s*token,\s*&investor\)',
                 r'is_blacklisted(&issuer, &symbol_short!("def"), &token, &investor)', content)

# Fix claim: (holder, namespace, token, period_count)
# Wait, claim signature in lib.rs is:
# pub fn claim(env: Env, holder: Address, issuer: Address, namespace: Symbol, token: Address) -> u32
# It now takes issuer and namespace.
content = re.sub(r'\.(try_)?claim\(&holder,\s*&token,\s*&0u32\)',
                 r'.\1claim(&holder, &issuer, &symbol_short!("def"), &token)', content)

with open(path, 'w') as f:
    f.write(content)
