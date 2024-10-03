#!/bin/bash

cd csv/

# Define the range of epochs
START_EPOCH=694
END_EPOCH=708

# Initialize the table with headers
printf "| Epoch | db-sync 13.3.0.0 | db-sync 13.5.0.2 |\n"
printf "|-------|------------------|------------------|\n"

# Loop through each epoch
for (( epoch=START_EPOCH; epoch<=END_EPOCH; epoch++ ))
do
    # Define file names
    blockfrost_file="blockfrost_${epoch}_stake.csv"
    dbsync_133_file="dbsync_13_3_0_0_${epoch}_stake.csv"
    dbsync_135_file="dbsync_13_5_0_2_${epoch}_stake.csv"

    # Initialize comparison results
    result_133="N/A"
    result_135="N/A"

    # Compare with db-sync 13.3.0.0 if file exists
    if [[ -f "$blockfrost_file" && -f "$dbsync_133_file" ]]; then
        if diff -q "$blockfrost_file" "$dbsync_133_file" > /dev/null; then
            result_133="-"
        else
            result_133="Diff"
        fi
    else
        result_133="File Missing"
    fi

    # Compare with db-sync 13.5.0.0 if file exists
    if [[ -f "$blockfrost_file" && -f "$dbsync_135_file" ]]; then
        if diff -q "$blockfrost_file" "$dbsync_135_file" > /dev/null; then
            result_135="-"
        else
            result_135="Diff"
        fi
    else
        result_135="File Missing"
    fi

    # Print the results in table format
    printf "| %-5d | %-16s | %-16s |\n" "$epoch" "$result_133" "$result_135"
done
