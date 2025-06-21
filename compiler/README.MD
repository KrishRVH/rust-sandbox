cargo build --release && \
time ./target/release/mist-compiler src/dice.mist --run && \
echo "Compiled size: $(ls -lh target/release/mist-compiler | awk '{print $5}')"


Could fix stochastic with nanosecond precision (currently it is too fast)

output.push_str("#include <time.h>\n");
output.push_str("#include <unistd.h>\n");  // Add this
// ...
output.push_str(&self.indent("// Initialize RNG for stochastic semantics\n"));
output.push_str(&self.indent("struct timespec ts;\n"));
output.push_str(&self.indent("clock_gettime(CLOCK_REALTIME, &ts);\n"));
output.push_str(&self.indent("srand(ts.tv_nsec ^ ts.tv_sec ^ getpid());\n\n"));