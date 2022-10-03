FROM --platform=linux/amd64 alpine:3.16
COPY ./target/x86_64-unknown-linux-gnu/release/health-probe /health-probe
CMD ["/health-probe"]
