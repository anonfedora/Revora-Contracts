import re
import os

path = '/home/edohwares/Desktop/Room/grantfox/Revora-Contracts/src/test.rs'

with open(path, 'r') as f:
    content = f.read()

# 1. Rename _issuer to issuer in common setup returns
content = content.replace('let (env, client, _issuer, token,', 'let (env, client, issuer, token,')
content = content.replace('let (env, client, _issuer, _token,', 'let (env, client, issuer, _token,')

# 2. General Namespace Injection for functions matching .func(&issuer, &token...
# List of functions that take (&issuer, &token, ...) and need NS in between.
funcs_needing_ns = [
    'get_claim_delay', 'set_claim_delay', 'try_set_claim_delay',
    'simulate_distribution', 'get_snapshot_config', 'set_snapshot_config',
    'get_pending_periods', 'get_claimable', 'get_holder_share',
    'get_period_count', 'deposit_revenue', 'try_deposit_revenue',
    'is_blacklisted', 'is_whitelisted', 'get_audit_summary',
    'get_rounding_mode', 'get_current_concentration', 'get_min_revenue_threshold',
    'get_concentration_limit', 'report_concentration', 'is_whitelist_enabled',
    'set_concentration_limit', 'set_rounding_mode', 'set_min_revenue_threshold',
    'set_offering_metadata', 'set_holder_share', 'try_set_holder_share'
]

for func in funcs_needing_ns:
    # Pattern: .func(&issuer, &token_name
    # Replace with: .func(&issuer, &symbol_short!("def"), &token_name
    content = re.sub(r'\.' + func + r'\(&issuer,\s*(&[a-z0_9_]*token[a-z0-9_]*)',
                     r'.' + func + r'(&issuer, &symbol_short!("def"), \1', content)

# Special case for claim/try_claim: (holder, issuer, NS, token, max_periods)
content = re.sub(r'\.(try_)?claim\((&holder[a-z0-9_]*),\s*&issuer,\s*&symbol_short!\("def"\),\s*(&token[a-z0-9_]*)\)',
                 r'.\1claim(\2, &issuer, &symbol_short!("def"), \3, &0)', content)

# 3. Fix blacklist/whitelist add/remove with CALLER
# These take 5 args: (caller, issuer, NS, token, investor)
for func in ['blacklist_add', 'blacklist_remove', 'whitelist_add', 'whitelist_remove']:
    # Fix from 4 args (issuer, NS, token, holder) to 5 (issuer, issuer, NS, token, holder)
    content = re.sub(r'\.' + func + r'\(&issuer,\s*&symbol_short!\("def"\),\s*&([a-z0-9_]+),\s*&([a-z0-9_]+)\)',
                     r'.' + func + r'(&issuer, &issuer, &symbol_short!("def"), &\1, &\2)', content)

# 4. Fix register_offering supply_cap
# Already mostly fixed but ensure 6 args.
content = re.sub(r'(\.register_offering\(&issuer,\s*&symbol_short!\("def"\),\s*&[a-z0-9_]+,\s*&[0-9,_]+,\s*&[a-z0-9_]+)\)', 
                 r'\1, &0)', content)

# 5. Fix payout assertions
content = content.replace('assert_eq!(payout, 50_000);', 'assert_eq!(payout_a, 50_000);')

# 6. Final cleanup
content = content.replace('&&', '&')
content = content.replace('&&', '&')
# Fix double NS injections
content = re.sub(r'&symbol_short!\("def"\),\s*&symbol_short!\("def"\),', r'&symbol_short!("def"),', content)

with open(path, 'w') as f:
    f.write(content)
