# State crate

This crate is implementing the SecondaryStorage and StorageView.

While most of the Micro data is currently stored in postgres - we also keep a secondary copy for part of it in RocksDB
for performance reasons.

Currently we only keep the data that is needed by the VM (which is why we implement MicroReadStorage for this
SecondaryStorage class).
