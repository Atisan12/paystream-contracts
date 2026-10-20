#!/usr/bin/env bash
#
# project-stream.sh — model payroll costs for a PayStream salary stream
#
# Usage: project-stream.sh <rate_per_second> <deposit> [stop_time] [--json]
#
# Given a stream's rate_per_second and deposit, computes:
#   - time to exhaustion (seconds and days)
#   - monthly cost (tokens per 30-day month)
#   - recommended deposit to fund 6 months of streaming
#
# stop_time is optional; when 0 or omitted the stream runs indefinitely.
# Pass --json to emit machine-readable JSON instead of plain text.

set -euo pipefail

JSON=0
RATE=""
DEPOSIT=""
STOP_TIME=""

while [ $# -gt 0 ]; do
    case "$1" in
        --json)
            JSON=1
            shift
            ;;
        --help|-h)
            echo "Usage: project-stream.sh <rate_per_second> <deposit> [stop_time] [--json]"
            exit 0
            ;;
        *)
            if [ -z "$RATE" ]; then
                RATE="$1"
            elif [ -z "$DEPOSIT" ]; then
                DEPOSIT="$1"
            else
                STOP_TIME="$1"
            fi
            shift
            ;;
    esac
done

if [ -z "$RATE" ] || [ -z "$DEPOSIT" ]; then
    echo "Usage: project-stream.sh <rate_per_second> <deposit> [stop_time] [--json]" >&2
    exit 1
fi

SEC_PER_MONTH=2592000   # 30 days in seconds
SEC_PER_DAY=86400
MONTHS=6

awk -v rate="$RATE" -v deposit="$DEPOSIT" -v stop_time="${STOP_TIME:-0}" \
    -v sec_per_month="$SEC_PER_MONTH" -v sec_per_day="$SEC_PER_DAY" \
    -v months="$MONTHS" -v json="$JSON" '
BEGIN {
    monthly = rate * sec_per_month;
    if (rate > 0) {
        exhaustion_sec = deposit / rate;
    } else {
        exhaustion_sec = -1;
    }
    exhaustion_days = exhaustion_sec / sec_per_day;
    rec6 = rate * sec_per_month * months;

    if (json == 1) {
        printf "{\n";
        printf "  \"rate_per_second\": %s,\n", rate;
        printf "  \"deposit\": %s,\n", deposit;
        printf "  \"stop_time\": %s,\n", stop_time;
        printf "  \"time_to_exhaustion_seconds\": %s,\n", exhaustion_sec;
        printf "  \"time_to_exhaustion_days\": %.2f,\n", exhaustion_days;
        printf "  \"monthly_cost\": %s,\n", monthly;
        printf "  \"recommended_deposit_6_months\": %s\n", rec6;
        printf "}\n";
    } else {
        printf "PayStream payroll projection\n";
        printf "============================\n";
        printf "Rate per second:    %s tokens\n", rate;
        printf "Deposit:            %s tokens\n", deposit;
        if (stop_time + 0 != 0) printf "Stop time:          %s\n", stop_time;
        printf "Time to exhaustion: %s seconds (~%.2f days)\n", exhaustion_sec, exhaustion_days;
        printf "Monthly cost:       %s tokens/month\n", monthly;
        printf "Recommended deposit for %d months: %s tokens\n", months, rec6;
    }
}
'
