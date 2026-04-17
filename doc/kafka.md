# Stringsimile kafka

Kafka support for stringsimile (input/output). Optional feature that
allows connection to kafka brokers for reading input data and/or writing
output data.

# Configuration

Check out [the configuration page](./configuration.md), for configuration options. Kafka
can be configured under `input` and `output` sections.

# Examples

## Kafka input and output

Here is an example of stringsimile configured
to read input data from kafka and also write to kafka under a different
topic.

    input:
      kafka:
        host: localhost
        port: 9092
        topics: ["input"]
        identifier: stringsimile
        pointer: "end" # "now" works too
        librdkafka_options: {}

    output:
      kafka:
        host: localhost
        port: 9092
        topic: stringsimile
        identifier: stringsimile
        librdkafka_options: {}

The above configuration connects to the same kafka broker for input and
output (localhost:9092). It reads from "input" topic (from the end
specifically, not from stored offset). It writes output data to the
"stringsimile" topic.

## Kafka input from saved offset

    input:
      kafka:
        host: localhost
        port: 9092
        topics: ["input"]
        identifier: stringsimile
        pointer: null
        librdkafka_options: {}

The above configuration connects to localhost:9092 for input data, reads
from "input" topic, from the stored offset, allowing this instance to
continue where it left off when it is restarted.

## Kafka input from beginning

    input:
      kafka:
        host: localhost
        port: 9092
        topics: ["input"]
        identifier: stringsimile
        pointer: "start" # "begin" works too
        librdkafka_options: {}

The above configuration connects to localhost:9092 for input data, reads
from "input" topic, from the first message.

# See also

- [Main docs](./README.md)
- [Configuration](./configuration.md)
