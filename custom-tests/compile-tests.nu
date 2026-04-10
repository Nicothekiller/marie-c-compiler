for i in (ls | where type == file | where name ends-with .c | get name) {
  ../target/debug/marie-c-compiler $i
}
