#!/usr/bin/env bash
# daily_seed.sh — Insert one day of realistic care-home seed data into the dev DB.
#
# Prerequisites:
#   brew install sqlcipher
#
# Setup (run once):
#   chmod +x /path/to/scripts/daily_seed.sh
#   crontab -e
#   # Add this line to run at 06:00 every day:
#   0 6 * * * /Users/jangminho/development/care-home/scripts/daily_seed.sh >> /tmp/care_seed.log 2>&1
#
# The app also auto-seeds today's data on startup (seed_today_if_needed in lib.rs),
# so this script is only needed when the app isn't launched every day.

set -e

DB_PATH="$HOME/Library/Application Support/care.jangminho.care-home/care_home.db"
KEY="sunshine_care_2024"
TODAY=$(date +%Y-%m-%d)

# Verify sqlcipher is available
if ! command -v sqlcipher &>/dev/null; then
  echo "[$(date)] ERROR: sqlcipher not found. Install with: brew install sqlcipher"
  exit 1
fi

# Check if today's data already exists
COUNT=$(sqlcipher "$DB_PATH" "PRAGMA key='$KEY'; SELECT COUNT(*) FROM care_logs WHERE date(logged_at)='$TODAY';" 2>/dev/null | grep -E '^[0-9]+$' | tail -1)
if [ "${COUNT:-0}" -ge 10 ]; then
  echo "[$(date)] Today's data already present ($COUNT entries). Skipping."
  exit 0
fi

# Fetch staff IDs (first 8)
STAFF_IDS=$(sqlcipher "$DB_PATH" "PRAGMA key='$KEY'; SELECT id FROM users WHERE role IN ('staff','manager') LIMIT 8;" 2>/dev/null | grep -E '^[0-9]+$')
STAFF_ARR=($STAFF_IDS)
N=${#STAFF_ARR[@]}
if [ $N -eq 0 ]; then
  echo "[$(date)] ERROR: No staff users found in DB."
  exit 1
fi

sid() { echo "${STAFF_ARR[$(( ($1) % N ))]}"; }

# Content arrays
BATHING=(
  "Full assistance with morning shower. Skin intact, no pressure areas. Moisturiser applied."
  "Bed bath completed — resident declined shower. Oral care done. No skin breakdown."
  "Assisted shower — resident managed upper body. Compression stockings reapplied post-shower."
  "Morning hygiene with full assist. Dentures cleaned. No concerns noted."
  "Two-staff assisted shower. Skin assessment done — no redness or concerns."
)
MEALS=(
  "Ate 80% of breakfast. Encouraged fluids — 300ml water and juice consumed."
  "Full lunch consumed. Appetite excellent. Fluid intake 450ml this shift."
  "Ate 60% of dinner. Mild appetite decrease noted."
  "Pureed diet consumed in full. No coughing. Good fluid intake."
  "Appetite good. Requested additional fruit. Fluids 400ml for the shift."
)
MEDICATION=(
  "Morning medications administered as prescribed. No refusals."
  "All medications taken without difficulty. BP within expected range."
  "Medications given as scheduled. Resident required gentle prompting."
  "Evening medications administered. Pain rated 2/10; PRN analgesia given."
  "Medications administered. One tablet given with apple juice after initial refusal."
)
MOOD=(
  "Resident in good spirits. Participated in morning group exercise."
  "Appeared slightly anxious. Redirected with music — settled after 15 minutes."
  "Calm and engaged. Positive interaction with fellow residents at lunch."
  "Good mood. Resident requested phone call with family — assisted."
  "Cheerful and talkative. Participated in afternoon reminiscence group."
)
NOTE=(
  "Repositioned every 2 hours. Skin integrity maintained throughout shift."
  "Resident ambulating with walker. Steady gait, no falls observed."
  "Physiotherapy exercises completed. Resident tolerated activity well."
  "Incident-free shift. Resident cooperative and settled throughout."
  "Night: resident called at 02:00 for toileting. Resettled promptly."
)
VISIT=(
  "Family visit — daughter present for ~1 hour. Resident appeared happy and engaged."
  "Dr. Chen visited: medication review completed. No changes to care plan."
  "Physiotherapy session: good progress with mobility and balance noted."
  "Family visit — son and grandchildren present 2 hours. Resident visibly delighted."
  "Dietitian review. Dietary plan updated for caloric intake goals."
)

# 15 residents — (resident_id, primary_cat_index: 0=bathing 1=meals 2=medication 3=mood 4=note)
RESIDENTS=(1:2 2:1 3:3 4:0 5:2 6:4 7:2 8:1 9:0 10:3 11:4 12:1 13:0 14:2 15:4)

INSERTS=""
RES_IDX=0

for ENTRY in "${RESIDENTS[@]}"; do
  RID=${ENTRY%%:*}
  FOCUS=${ENTRY##*:}

  H=$(( (200 * 17) + (RES_IDX * 7) ))   # offset=200 matches seed_today_if_needed
  CAT_IDX=$(( H % 4 == 0 ? FOCUS : H % 5 ))
  HOUR=$(( 6 + H % 14 ))

  case $CAT_IDX in
    0) CAT="bathing";    CONTENT="${BATHING[$(( H % ${#BATHING[@]} ))]}" ;;
    1) CAT="meals";      CONTENT="${MEALS[$(( H % ${#MEALS[@]} ))]}" ;;
    2) CAT="medication"; CONTENT="${MEDICATION[$(( H % ${#MEDICATION[@]} ))]}" ;;
    3) CAT="mood";       CONTENT="${MOOD[$(( H % ${#MOOD[@]} ))]}" ;;
    *)  CAT="note";       CONTENT="${NOTE[$(( H % ${#NOTE[@]} ))]}" ;;
  esac

  SID=$(sid $H)
  # Escape single quotes in content
  CONTENT_ESC="${CONTENT//\'/\'\'}"
  INSERTS+="INSERT INTO care_logs (resident_id,staff_id,shift,category,content,is_incident,logged_at) VALUES ($RID,$SID,'day','$CAT','$CONTENT_ESC',0,datetime('now','+'$HOUR' hours'));"

  # Visit entry for some residents
  if (( RES_IDX % 7 == 0 )); then
    VH=$(( H + 5 ))
    VCONTENT="${VISIT[$(( VH % ${#VISIT[@]} ))]}"
    VCONTENT_ESC="${VCONTENT//\'/\'\'}"
    VSID=$(sid $VH)
    INSERTS+="INSERT INTO care_logs (resident_id,staff_id,shift,category,content,is_incident,logged_at) VALUES ($RID,$VSID,'visit','note','$VCONTENT_ESC',0,datetime('now','+14 hours'));"
  fi

  RES_IDX=$(( RES_IDX + 1 ))
done

# Execute all inserts in one transaction
sqlcipher "$DB_PATH" <<EOF
PRAGMA key='$KEY';
BEGIN;
$INSERTS
COMMIT;
EOF

echo "[$(date)] Seeded care log entries for $TODAY."
