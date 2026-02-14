# Advanced Examples & Enterprise Workflows

Real-world examples and automation patterns for domain-check in professional environments.

## Table of Contents

- [Developer Workflows](#developer-workflows)
- [Simple Automation](#simple-automation)
- [Project & Product Planning](#project--product-planning)
- [CI/CD Integration](#cicd-integration)
- [Data Processing](#data-processing)
- [Advanced Enterprise Scenarios](#advanced-enterprise-scenarios)

- [Domain Generation](#domain-generation)

---

## Domain Generation

### Pattern-Based Domain Discovery

Find available domains using wildcard patterns — no external tools needed.

```bash
# Explore all 3-letter .com domains (dry run first to see count)
domain-check --pattern "\w\w\w" -t com --dry-run 2>&1 | tail -1
# 19683 domains would be checked

# Actually check a smaller pattern
domain-check --pattern "go\d" -t com,io --batch --json > go-domains.json

# Find available 4-letter domains starting with "ai"
domain-check --pattern "ai\w\w" -t com --yes --json | \
  jq -r '.[] | select(.available==true) | .domain'
```

### Prefix/Suffix Brand Exploration

```bash
# Explore branding options for "cloud"
domain-check cloud --prefix get,my,try,use --suffix hub,ly,app -t com --dry-run
# 16 domains would be checked

# Check them for real
domain-check cloud --prefix get,my,try --suffix hub,ly -t com,io --pretty --batch
```

### AI Agent / Automation Integration

domain-check is designed to be composable with AI agents and automation pipelines:

```bash
# Non-interactive: --yes skips confirmation, --json gives structured output
domain-check --pattern "app\d" --prefix get -t com --yes --json | \
  jq -r '.[] | select(.available==true) | .domain'

# Piped output never prompts (non-TTY detection)
domain-check --pattern "test\d\d" -t com --json | jq length

# Dry-run for cost estimation before committing
COUNT=$(domain-check --pattern "x\d\d" --prefix get,my --preset startup --dry-run 2>&1 | \
  grep "domains would" | grep -o '[0-9]*')
echo "Would check $COUNT domains"

# Combine with file input + generation
echo -e "myapp\ncoolsite" > names.txt
domain-check --file names.txt --prefix get,try --suffix hub -t com --dry-run
```

### Team Workflow with Config Defaults

Set up persistent generation defaults for your team:

```toml
# .domain-check.toml (commit to repo)
[defaults]
concurrency = 30
preset = "startup"
pretty = true

[generation]
prefixes = ["get", "my", "try"]
suffixes = ["hub", "app", "ly"]
```

```bash
# Every team member automatically gets prefix/suffix expansion
domain-check newfeature -t com
# → getnewfeature.com, mynewfeature.com, trynewfeature.com,
#   newfeaturehub.com, newfeatureapp.com, newfeaturely.com,
#   newfeature.com

# Override with CLI flags when needed
domain-check newfeature --prefix super --suffix ai -t com
# → supernewfeature.com, supernewfeatureai.com, newfeatureai.com, newfeature.com

# Environment variables work too
DC_PREFIX=cool,hot domain-check mysite -t com --dry-run
```

---

## Developer Workflows

### Quick Domain Research for Side Projects

Every developer needs to check domain availability when starting new projects. Here's a streamlined workflow.

```bash
# 1. Check your project name across essential TLDs
domain-check myawesomeapp --preset startup --pretty
# 🔴 myawesomeapp.com is TAKEN
# 🟢 myawesomeapp.io is AVAILABLE  
# 🟢 myawesomeapp.dev is AVAILABLE
# 🟢 myawesomeapp.app is AVAILABLE

# 2. Check variations if the main name is taken
echo "myawesomeapp
awesome-app  
my-awesome-app" > app-names.txt

domain-check --file app-names.txt -t com,io,dev --streaming
```

### Pre-Purchase Domain Validation

Before buying domains, verify they're actually available and get detailed info.

```bash
# 1. Verify specific domains you want to buy
domain-check myapp.com myapp.io myapp.dev --info --pretty
# 🔴 myapp.com is TAKEN (Registrar: GoDaddy, Expires: 2025-12-15)
# 🟢 myapp.io is AVAILABLE
# 🟢 myapp.dev is AVAILABLE

# 2. Export results for decision making
domain-check myapp.com myapp.io myapp.dev --info --csv > purchase-decision.csv
```

### API/Service Domain Planning

When building APIs or microservices, plan your domain structure systematically.

```bash
# 1. Create service domain list
cat > api-domains.txt << 'EOF'
api.myapp
auth.myapp  
admin.myapp
docs.myapp
status.myapp
blog.myapp
EOF

# 2. Check if subdomains are available as separate domains
domain-check --file api-domains.txt -t com,io --json > service-domains.json

# 3. Find available alternatives
jq -r '.[] | select(.available==true) | .domain' service-domains.json
```

---

## Simple Automation

### Daily Domain Monitoring Script

A simple script to monitor domains you're interested in buying.

```bash
#!/bin/bash
# daily-check.sh - Simple domain monitoring

WATCH_LIST="watch-domains.txt"

echo "🔍 Daily domain check - $(date)"

# Check your watchlist
domain-check --file "$WATCH_LIST" --preset startup --pretty

# Save results with date
domain-check --file "$WATCH_LIST" --preset startup --csv > "results-$(date +%Y%m%d).csv"

echo "📊 Results saved to results-$(date +%Y%m%d).csv"
```

Create your watch list:
```bash
echo "coolstartup
awesome-saas
my-next-project" > watch-domains.txt

# Run daily check
./daily-check.sh
```

### Batch Domain Research

Research multiple project ideas efficiently.

```bash
#!/bin/bash
# batch-research.sh

echo "📝 Enter your project ideas (one per line, Ctrl+D when done):"
cat > project-ideas.txt

echo "🔍 Checking availability across startup TLDs..."
domain-check --file project-ideas.txt --preset startup --streaming --csv > research-results.csv

echo "✅ Available .com domains:"
grep "\.com,true," research-results.csv | cut -d',' -f1

echo "✅ Available .io domains:"  
grep "\.io,true," research-results.csv | cut -d',' -f1

echo "📊 Full results: research-results.csv"
```

### GitHub Action for Domain Validation

Simple CI check to ensure your project domains are registered before deployment.

```yaml
# .github/workflows/domain-check.yml
name: Validate Domains

on: [push, pull_request]

jobs:
  check-domains:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install domain-check
        run: cargo install domain-check
        
      - name: Check required domains
        run: |
          echo "myapp.com
          api.myapp.com  
          docs.myapp.com" > required-domains.txt
          
          domain-check --file required-domains.txt --json > domain-status.json
          
          # Fail if any required domain is available (should be registered)
          if jq -e '.[] | select(.available==true)' domain-status.json; then
            echo "❌ Some required domains are not registered!"
            exit 1
          fi
          
          echo "✅ All required domains are registered"
```

---

## Project & Product Planning

### Startup Name Research

Systematic approach to finding and evaluating startup names.

```bash
#!/bin/bash
# startup-name-research.sh

echo "🚀 Startup Name Research Tool"
echo "Enter potential names (one per line, Ctrl+D when done):"
cat > startup-names.txt

# 1. Check core availability
echo "🔍 Checking startup-focused TLDs..."
domain-check --file startup-names.txt --preset startup --pretty

# 2. Generate comprehensive report
domain-check --file startup-names.txt --preset startup --csv > startup-analysis.csv

# 3. Show best options (available .com or .io)
echo ""
echo "🎯 Best Available Options:"
echo "📍 .com domains:"
grep "\.com,true," startup-analysis.csv | cut -d',' -f1 | sed 's/^/  ✅ /'

echo "📍 .io domains:"
grep "\.io,true," startup-analysis.csv | cut -d',' -f1 | sed 's/^/  ✅ /'

# 4. Cost estimation
COM_COUNT=$(grep "\.com,true," startup-analysis.csv | wc -l)
IO_COUNT=$(grep "\.io,true," startup-analysis.csv | wc -l)

echo ""
echo "💰 Cost Estimation:"
echo "  .com domains: $COM_COUNT × \$15/year = \$(($COM_COUNT * 15))"
echo "  .io domains: $IO_COUNT × \$50/year = \$(($IO_COUNT * 50))"
```

### Product Launch Domain Strategy

Plan domain acquisitions for product launches.

```bash
#!/bin/bash
# product-launch.sh

PRODUCT_NAME="$1"
if [ -z "$PRODUCT_NAME" ]; then
  echo "Usage: $0 <product-name>"
  exit 1
fi

echo "🚀 Product Launch Domain Strategy: $PRODUCT_NAME"

# 1. Generate essential domains
cat > product-domains.txt << EOF
${PRODUCT_NAME}
get${PRODUCT_NAME}
try${PRODUCT_NAME}
${PRODUCT_NAME}app
${PRODUCT_NAME}-api
${PRODUCT_NAME}-docs
EOF

# 2. Check availability
domain-check --file product-domains.txt -t com,io,app --pretty

# 3. Priority recommendations
echo ""
echo "📋 Acquisition Priorities:"
echo "1. ${PRODUCT_NAME}.com (if available)"
echo "2. ${PRODUCT_NAME}.io (tech credibility)"  
echo "3. ${PRODUCT_NAME}.app (mobile-first)"
echo "4. get${PRODUCT_NAME}.com (marketing)"

# 4. Export for team review
domain-check --file product-domains.txt -t com,io,app --csv > "${PRODUCT_NAME}-domains.csv"
echo "📊 Detailed analysis: ${PRODUCT_NAME}-domains.csv"
```

Usage:
```bash
./product-launch.sh "superapp"
```

### Brand Protection Basics

Simple monitoring for personal or small business brands.

```bash
#!/bin/bash
# brand-monitor.sh

BRAND="$1"
if [ -z "$BRAND" ]; then
  echo "Usage: $0 <brand-name>"
  exit 1
fi

echo "🛡️ Brand Protection Check: $BRAND"

# 1. Generate common brand variations
cat > brand-variations.txt << EOF
${BRAND}
${BRAND}-app
${BRAND}-api
${BRAND}-shop
${BRAND}-store
${BRAND}-official
fake-${BRAND}
${BRAND}-fake
EOF

# 2. Check across business TLDs
domain-check --file brand-variations.txt --preset enterprise --pretty

# 3. Flag potential issues
echo ""
echo "⚠️ Monitor these registered variations:"
domain-check --file brand-variations.txt --preset enterprise --json | \
  jq -r '.[] | select(.available==false) | .domain' | \
  grep -v "^${BRAND}\\.com$" | \
  sed 's/^/  🚨 /'

echo ""
echo "✅ Consider registering these available variations:"
domain-check --file brand-variations.txt --preset enterprise --json | \
  jq -r '.[] | select(.available==true) | .domain' | \
  sed 's/^/  💡 /'
```

---

## CI/CD Integration

### Simple Pre-Deployment Check

Ensure critical domains are registered before deployment.

```bash
#!/bin/bash
# pre-deploy-check.sh

echo "🔍 Pre-deployment domain validation..."

# 1. Extract domains from your app config (example)
DOMAINS=$(grep -h "domain:" config/*.yml | awk '{print $2}' | sort -u)

# 2. Check if they're all registered
echo "$DOMAINS" > deployment-domains.txt
domain-check --file deployment-domains.txt --json > domain-status.json

# 3. Validate all are registered
UNREGISTERED=$(jq -r '.[] | select(.available==true) | .domain' domain-status.json)

if [ -n "$UNREGISTERED" ]; then
  echo "❌ DEPLOYMENT BLOCKED: Unregistered domains found:"
  echo "$UNREGISTERED"
  exit 1
else
  echo "✅ All deployment domains are registered"
fi
```

### Docker Integration

Use domain-check in Docker containers for portable checks.

```dockerfile
# Dockerfile
FROM rust:slim

RUN cargo install domain-check

WORKDIR /app
COPY domains.txt .

CMD ["domain-check", "--file", "domains.txt", "--preset", "startup", "--json"]
```

```bash
# Build and run
docker build -t domain-checker .
docker run --rm domain-checker > results.json
```

---

## Data Processing

### Find Available Domains with jq

Process JSON results to find specific opportunities.

```bash
# 1. Check many domains
domain-check startup unicorn awesome-app cool-tech --all --json > scan.json

# 2. Find available .com domains
jq -r '.[] | select(.available==true and (.domain | endswith(".com"))) | .domain' scan.json

# 3. Group by TLD
jq -r '.[] | select(.available==true) | .domain' scan.json | \
  awk -F. '{print $NF}' | sort | uniq -c | sort -nr

# 4. Find domains checked via RDAP (most reliable)
jq -r '.[] | select(.method_used=="rdap" and .available==true) | .domain' scan.json
```

### Generate Reports with awk

Create simple reports from CSV output.

```bash
# 1. Generate CSV data
domain-check --file startup-ideas.txt --preset startup --csv > results.csv

# 2. Summary by TLD
echo "TLD Availability Summary:"
awk -F, 'NR>1 {split($1,parts,"."); tld=parts[length(parts)]; if($2=="true") available[tld]++; total[tld]++} 
END {for(t in total) printf "%s: %d/%d available\n", t, available[t]+0, total[t]}' results.csv

# 3. List available premium domains
echo "Available Premium Domains:"
awk -F, '$2=="true" && $1 ~ /\.(com|io)$/ {print "✅ " $1}' results.csv
```

### Simple Analytics

Basic analysis of domain checking results.

```bash
#!/bin/bash
# domain-analytics.sh

RESULTS_FILE="$1"
if [ ! -f "$RESULTS_FILE" ]; then
  echo "Usage: $0 <results.json>"
  exit 1
fi

echo "📊 Domain Analysis Report"
echo "========================"

TOTAL=$(jq length "$RESULTS_FILE")
AVAILABLE=$(jq '[.[] | select(.available==true)] | length' "$RESULTS_FILE") 
TAKEN=$(jq '[.[] | select(.available==false)] | length' "$RESULTS_FILE")

echo "Total domains checked: $TOTAL"
echo "Available: $AVAILABLE ($(echo "scale=1; $AVAILABLE * 100 / $TOTAL" | bc)%)"
echo "Taken: $TAKEN ($(echo "scale=1; $TAKEN * 100 / $TOTAL" | bc)%)"

echo ""
echo "🎯 Top Available TLDs:"
jq -r '.[] | select(.available==true) | .domain' "$RESULTS_FILE" | \
  awk -F. '{print $NF}' | sort | uniq -c | sort -nr | head -5

echo ""
echo "✅ Best Available Domains:"
jq -r '.[] | select(.available==true and (.domain | test("\\.(com|io|dev)$"))) | .domain' "$RESULTS_FILE" | head -10
```

---

## Advanced Enterprise Scenarios

*For organizations needing large-scale domain management and monitoring.*

### Enterprise Domain Portfolio Audit

```bash
#!/bin/bash
# enterprise-audit.sh

echo "🏢 Enterprise Domain Portfolio Audit"

# 1. Create comprehensive domain list
cat > enterprise-domains.txt << 'EOF'
# Primary brands
acmecorp
acme-corp
acmetech

# Product lines
acme-cloud
acme-security
acme-analytics

# Geographic variants
acme-europe
acme-asia
acme-americas
EOF

# 2. Multi-region audit
domain-check --file enterprise-domains.txt \
  --preset enterprise \
  --concurrency 30 \
  --csv > audit-$(date +%Y%m%d).csv

# 3. Generate executive summary
echo "📊 Executive Summary:" > audit-summary.txt
echo "Date: $(date)" >> audit-summary.txt

for tld in com org net; do
  count=$(grep "\\.$tld,true," audit-$(date +%Y%m%d).csv | wc -l)
  echo "Available .$tld domains: $count" >> audit-summary.txt
done
```

### Brand Protection Monitoring

```bash
#!/bin/bash
# brand-monitor-enterprise.sh

BRAND="acmecorp"
SLACK_WEBHOOK="your-webhook-url"

# Generate brand variations
cat > brand-variations.txt << EOF
${BRAND}
${BRAND}-app
${BRAND}-store
fake-${BRAND}
${BRAND}-fake
${BRAND}-official
EOF

# Check all TLDs
domain-check --file brand-variations.txt --all --json > brand-scan.json

# Alert on new registrations
if [ -f "previous-scan.json" ]; then
  NEW_THREATS=$(comm -13 \
    <(jq -r '.[] | select(.available==false) | .domain' previous-scan.json | sort) \
    <(jq -r '.[] | select(.available==false) | .domain' brand-scan.json | sort))
  
  if [ -n "$NEW_THREATS" ]; then
    curl -X POST -H 'Content-type: application/json' \
      --data "{\"text\":\"🚨 New brand threats: $NEW_THREATS\"}" \
      "$SLACK_WEBHOOK"
  fi
fi

cp brand-scan.json previous-scan.json
```

### Database Integration

```bash
#!/bin/bash
# database-integration.sh

# Check domains and store in database
domain-check --file important-domains.txt --preset enterprise --json > scan.json

# Insert into PostgreSQL (example)
jq -r '.[] | [.domain, .available, .method_used] | @csv' scan.json | \
while IFS=',' read -r domain available method; do
  psql -d domains -c "INSERT INTO scans (domain, available, method, scan_date) 
                      VALUES ($domain, $available, $method, NOW());"
done
```

---

---

## Universal TLD Coverage

With bootstrap enabled by default, domain-check can check domains across 1,300+ TLDs — virtually every TLD on the internet.

### Full TLD Scan

```bash
# Scan a brand across every known TLD
domain-check mybrand --all --json > full-scan.json

# How many TLDs are available?
jq '[.[] | select(.available==true)] | length' full-scan.json

# Group results by availability
jq -r '.[] | select(.available==true) | .domain' full-scan.json | wc -l
jq -r '.[] | select(.available==false) | .domain' full-scan.json | wc -l
```

### Checking Uncommon TLDs

Bootstrap handles TLDs that aren't in the hardcoded list — no special flags needed.

```bash
# These all work automatically via IANA bootstrap + WHOIS discovery
domain-check example.museum
domain-check example.photography
domain-check example.restaurant
domain-check mybrand -t com,io,dev,restaurant,photography,museum

# For TLDs without RDAP, WHOIS server is discovered via IANA referral
domain-check example.es    # .es has no RDAP — uses WHOIS automatically
domain-check example.co    # .co WHOIS discovered via whois.iana.org
```

### Offline / Restricted Mode

```bash
# Disable bootstrap for deterministic, offline-capable checks
# (limited to 32 hardcoded TLDs with known RDAP endpoints)
domain-check myapp --all --no-bootstrap

# Useful for CI environments with network restrictions
DC_BOOTSTRAP=false domain-check --file domains.txt --preset startup --json
```

### Pre-warming the Bootstrap Cache

```bash
# The bootstrap cache is fetched on first use and cached for 24 hours.
# For latency-sensitive workflows, the --all flag pre-warms the cache
# before checking, so subsequent calls are instant.
domain-check mybrand --all --json > /dev/null   # warms cache
domain-check otherbrand --all --json             # uses cached data
```

---

These examples showcase domain-check's versatility in enterprise environments, from simple automation to complex integration patterns. Each workflow can be adapted to specific organizational needs and integrated with existing tools and processes.

*For more library integration examples, see the [Library Documentation](https://docs.rs/domain-check-lib).*