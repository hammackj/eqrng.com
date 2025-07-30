#!/bin/bash

# Verification script for Barbarian and Iksar races
echo "🔍 EverQuest RNG Race Verification Script"
echo "=========================================="

# Check if the server is running
echo "1. Checking if server is running on port 3000..."
if lsof -i:3000 > /dev/null 2>&1; then
    echo "✅ Server is running on port 3000"
else
    echo "❌ Server is not running on port 3000"
    echo "Starting server..."
    cd "$(dirname "$0")"
    cargo run &
    SERVER_PID=$!
    echo "Server started with PID: $SERVER_PID"
    echo "Waiting 10 seconds for server to initialize..."
    sleep 10
fi

echo ""
echo "2. Verifying race images exist in dist directory..."
BARBARIAN_MALE="dist/assets/images/races/barbarian-male.png"
BARBARIAN_FEMALE="dist/assets/images/races/barbarian-female.png"
IKSAR_MALE="dist/assets/images/races/iksar-male.png"
IKSAR_FEMALE="dist/assets/images/races/iksar-female.png"

if [ -f "$BARBARIAN_MALE" ]; then
    echo "✅ Barbarian male image found"
else
    echo "❌ Barbarian male image missing"
fi

if [ -f "$BARBARIAN_FEMALE" ]; then
    echo "✅ Barbarian female image found"
else
    echo "❌ Barbarian female image missing"
fi

if [ -f "$IKSAR_MALE" ]; then
    echo "✅ Iksar male image found"
else
    echo "❌ Iksar male image missing"
fi

if [ -f "$IKSAR_FEMALE" ]; then
    echo "✅ Iksar female image found"
else
    echo "❌ Iksar female image missing"
fi

echo ""
echo "3. Testing random race API endpoint..."
echo "Generating 20 random races to check for new races..."

BARBARIAN_COUNT=0
IKSAR_COUNT=0
TOTAL_TESTS=20

for i in $(seq 1 $TOTAL_TESTS); do
    RESPONSE=$(curl -s http://localhost:3000/random_race 2>/dev/null)

    if [ $? -eq 0 ] && [ -n "$RESPONSE" ]; then
        # Check if response contains Barbarian or Iksar
        if echo "$RESPONSE" | grep -q "Barbarian"; then
            BARBARIAN_COUNT=$((BARBARIAN_COUNT + 1))
            GENDER=$(echo "$RESPONSE" | grep -o '"gender":"[^"]*"' | cut -d'"' -f4)
            echo "  Found: Barbarian ($GENDER)"
        fi

        if echo "$RESPONSE" | grep -q "Iksar"; then
            IKSAR_COUNT=$((IKSAR_COUNT + 1))
            GENDER=$(echo "$RESPONSE" | grep -o '"gender":"[^"]*"' | cut -d'"' -f4)
            echo "  Found: Iksar ($GENDER)"
        fi
    else
        echo "❌ API call $i failed"
    fi
done

echo ""
echo "4. Results Summary:"
echo "==================="
echo "Total API calls: $TOTAL_TESTS"
echo "Barbarian races generated: $BARBARIAN_COUNT"
echo "Iksar races generated: $IKSAR_COUNT"

if [ $BARBARIAN_COUNT -gt 0 ]; then
    echo "✅ Barbarian race is working correctly!"
else
    echo "⚠️  Barbarian race not generated in this sample (try running more tests)"
fi

if [ $IKSAR_COUNT -gt 0 ]; then
    echo "✅ Iksar race is working correctly!"
else
    echo "⚠️  Iksar race not generated in this sample (try running more tests)"
fi

echo ""
echo "5. Testing specific image URLs..."
echo "Testing if images are accessible via HTTP..."

for image in "barbarian-male" "barbarian-female" "iksar-male" "iksar-female"; do
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:3000/assets/images/races/${image}.png" 2>/dev/null)

    if [ "$HTTP_CODE" = "200" ]; then
        echo "✅ ${image}.png is accessible (HTTP $HTTP_CODE)"
    else
        echo "❌ ${image}.png failed (HTTP $HTTP_CODE)"
    fi
done

echo ""
echo "6. Sample API Response:"
echo "======================"
SAMPLE_RESPONSE=$(curl -s http://localhost:3000/random_race 2>/dev/null)
if [ $? -eq 0 ] && [ -n "$SAMPLE_RESPONSE" ]; then
    echo "$SAMPLE_RESPONSE" | jq . 2>/dev/null || echo "$SAMPLE_RESPONSE"
else
    echo "❌ Failed to get sample response"
fi

echo ""
echo "🎯 Verification Complete!"
echo ""
echo "Next steps:"
echo "- Open http://localhost:3000/race in your browser"
echo "- Click 'Generate Random Race' multiple times"
echo "- Look for Barbarian and Iksar races with their images"
echo ""
echo "If you still don't see images, check the browser's developer tools (F12)"
echo "and look for any network errors in the Console or Network tabs."

# Clean up if we started the server
if [ ! -z "$SERVER_PID" ]; then
    echo ""
    echo "Cleaning up server process (PID: $SERVER_PID)..."
    kill $SERVER_PID 2>/dev/null
fi
