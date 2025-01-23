#!/bin/bash
extract_signals() {
  local filename="$1"

  # Check if the file exists
  if [[ ! -f "$filename" ]]; then
    echo "Error: File '$filename' does not exist."
    return 1
  fi

  # Use sed to extract JSON from each line in the file
  sed -n 's/.*-\({.*}\)$/\1/p' "$filename"
}

list_journeys() {
  local filename="$1"

  if [[ ! -f "$filename" ]]; then
    echo "Error: File '$filename' does not exist."
    return 1
  fi
  local signals=$(extract_signals "$filename") 
  local journeys=$(echo "${signals}"| jq -s . | jq '[.[] | select(.log_signal.message == "start_processing_request")]')
  echo "${journeys}" |  jq '[.[] | select(.log_signal.message == "start_processing_request")]' | jq '[.[] | {session_id: .log_signal.call_context.call_context.session_id, request_id: .log_signal.call_context.call_context.request_id, method: .log_signal.call_context.call_context.method}]' 
}
show_journey() {
  local filename="$1"
  local request_id="$2"

  if [[ ! -f "$filename" ]]; then
    echo "Error: File '$filename' does not exist."
    return 1
  fi

  local signals=$(extract_signals "$filename")
  local journey=$(echo "${signals}" | jq -s . | jq '[.[] | select(.log_signal.call_context.request_id == "'"$request_id"'")]')
  echo "${journey}"
}


main() {
  # Ensure at least one argument is provided
  if [[ "$#" -lt 2 ]]; then
    echo "Usage: $0 <command> <filename> [additional_args]"
    echo "Commands:"
    echo "  extract_signals <filename>"
    echo "  list_journeys <filename>"
    echo "  show_journey <filename> <request_id>"
    exit 1
  fi

  # Parse the command and call the corresponding function
  local command="$1"
  shift

  case "$command" in
    extract)
      extract_signals "$@"
      ;;
    list)
      list_journeys "$@"
      ;;
    show)
      show_journey "$@"
      ;;
    *)
      echo "Error: Unknown command '$command'"
      echo "Available commands: extract <ripple logfile path>, list <ripple logfile path>, show <ripple logfile path> <request_id>"
      exit 1
      ;;
  esac
}

# Call the main function with all the script arguments
main "$@"
