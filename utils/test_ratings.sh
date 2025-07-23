#!/bin/bash

# Test script for the zone rating API
# Make sure the server is running with: cargo run

BASE_URL="http://localhost:3000"
TEST_IP="192.168.1.100"

echo "🧪 Testing Zone Rating API"
echo "=========================="

# First, get a random zone to test with
echo "📍 Getting a random zone..."
ZONE_RESPONSE=$(curl -s "$BASE_URL/random_zone")
ZONE_ID=$(echo "$ZONE_RESPONSE" | grep -o '"id":[0-9]*' | cut -d':' -f2)
ZONE_NAME=$(echo "$ZONE_RESPONSE" | grep -o '"name":"[^"]*"' | cut -d'"' -f4)

if [ -z "$ZONE_ID" ]; then
    echo "❌ Failed to get a random zone"
    exit 1
fi

echo "✅ Got zone: $ZONE_NAME (ID: $ZONE_ID)"
echo ""

# Test 1: Get initial rating (should be empty)
echo "🔍 Test 1: Getting initial rating for zone $ZONE_ID..."
RATING_RESPONSE=$(curl -s "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP")
echo "Response: $RATING_RESPONSE"
echo ""

# Test 2: Submit a rating
echo "⭐ Test 2: Submitting a 4-pickle rating..."
SUBMIT_RESPONSE=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"rating": 4}' \
    "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP")
echo "Response: $SUBMIT_RESPONSE"
echo ""

# Test 3: Get rating again (should show our rating)
echo "🔍 Test 3: Getting rating after submission..."
RATING_RESPONSE2=$(curl -s "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP")
echo "Response: $RATING_RESPONSE2"
echo ""

# Test 4: Submit another rating from different IP
echo "⭐ Test 4: Submitting a 5-pickle rating from different user..."
TEST_IP2="192.168.1.101"
SUBMIT_RESPONSE2=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"rating": 5}' \
    "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP2")
echo "Response: $SUBMIT_RESPONSE2"
echo ""

# Test 5: Get aggregated ratings
echo "📊 Test 5: Getting aggregated ratings..."
RATING_RESPONSE3=$(curl -s "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP")
echo "Response: $RATING_RESPONSE3"
echo ""

# Test 6: Update existing rating
echo "🔄 Test 6: Updating existing rating to 3 pickles..."
SUBMIT_RESPONSE3=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"rating": 3}' \
    "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP")
echo "Response: $SUBMIT_RESPONSE3"
echo ""

# Test 7: Get all ratings for the zone (admin endpoint)
echo "🔍 Test 7: Getting all ratings for zone (admin view)..."
ALL_RATINGS=$(curl -s "$BASE_URL/zones/$ZONE_ID/ratings")
echo "Response: $ALL_RATINGS"
echo ""

# Test 8: Invalid rating (should fail)
echo "❌ Test 8: Trying to submit invalid rating (6 pickles)..."
INVALID_RESPONSE=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"rating": 6}' \
    "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP" \
    -w "HTTP_CODE:%{http_code}")
echo "Response: $INVALID_RESPONSE"
echo ""

# Test 9: Invalid rating (0 pickles)
echo "❌ Test 9: Trying to submit invalid rating (0 pickles)..."
INVALID_RESPONSE2=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"rating": 0}' \
    "$BASE_URL/zones/$ZONE_ID/rating?user_ip=$TEST_IP" \
    -w "HTTP_CODE:%{http_code}")
echo "Response: $INVALID_RESPONSE2"
echo ""

echo "🎉 Rating API tests completed!"
echo ""
echo "💡 To test the frontend:"
echo "   1. Start the backend: cargo run"
echo "   2. Start the frontend: cd frontend && npm run dev"
echo "   3. Visit http://localhost:4321"
echo "   4. Generate a random zone and try rating it with pickles!"
