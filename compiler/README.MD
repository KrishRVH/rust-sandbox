cargo build --release && \
time ./target/release/mist-compiler src/dice.mist --run && \
echo "Compiled size: $(ls -lh target/release/mist-compiler | awk '{print $5}')"