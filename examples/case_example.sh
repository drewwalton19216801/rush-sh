#!/usr/bin/env rush-sh

# Example case statement with glob patterns

echo "Testing case with glob patterns"
echo

# Test 1: File extension matching
echo "Test 1: File type detection"
case document.txt in
    *.txt|*.md) echo "document.txt is a text file" ;;
    *.jpg|*.png) echo "document.txt is an image" ;;
    *) echo "document.txt is something else" ;;
esac
echo

# Test 2: Single character wildcard
echo "Test 2: Single character match"
case file1 in
    file?) echo "file1 matches file?" ;;
    *) echo "file1 doesn't match" ;;
esac
echo

# Test 3: Character class
echo "Test 3: Character class [abc]"
case b in
    [abc]) echo "b is a, b, or c" ;;
    *) echo "b is not a, b, or c" ;;
esac
echo

# Test 4: Multiple patterns in one case
echo "Test 4: Multiple patterns"
case test.txt in
    *.txt|*.sh|*.md) echo "test.txt is a script or text file" ;;
    *.exe|*.bin) echo "test.txt is an executable" ;;
    *) echo "test.txt is unknown" ;;
esac
echo

# Test 5: Default case
echo "Test 5: Default case"
case random.stuff in
    *.txt) echo "Text file" ;;
    *.jpg) echo "Image" ;;
    *) echo "random.stuff falls to default case" ;;
esac