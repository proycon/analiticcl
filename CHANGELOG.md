
# v0.4.9 - 2026-01-05

## Python Binding

* updated dependencies (pyo3 v0.27.2)
* build wheels for Python 3.14

## In Memoriam

🦊 Martin Reynaert (1963-2025), whose research lies at the foundation of this software, passed away september 24th 2025. This is the first release since and this software will henceforth be in his memory.





# v0.4.8 - 2025-03-03

* added serde support
* Added Python Include file for documentation + fix for sdist build






# v0.4.7 - 2024-10-16

* Updated various dependencies, python binding now also runs on Python 3.13





# v0.4.6 - 2024-04-22

Minor update, dependency upgrades and added VariantModel.set_confusables_before_pruning() method to the Python binding that sets the `--early-confusables` parameter ([#19](https://github.com/proycon/analiticcl/issues/19))





# v0.4.5 - 2023-02-21

Bugfix release:

* Fixed bug in handling (hashing/normalizing) multibyte characters.
* Added a `testinput` mode to test input against the alphabet.
* Debug level two or higher now outputs the entire alphabet.





# v0.4.4 - 2022-08-08

Bugfix release:

*   fix in reading context rules





# v0.4.3 - 2022-08-08

Bugfix release:

* fix in reading context rules
* improving error feedback when parsing context rules
* added missing --contextrules parameter






# v0.4.2 - 2022-07-26

* A single context rule may now output multiple tags (and corresponding sequence numbers) (knaw-huc/golden-agents-htr#22)
* updated dependencies (e.g rustfst 0.11.5)






# v0.4.1 - 2022-06-17

Bugfixes:

* Fixed non-deterministic behaviour (in ties where scores were equal and in the ordering of anagram instances)





# v0.4.0 - 2022-05-10

New:
* Context rules and tagging (https://github.com/knaw-huc/golden-agents-htr#7): allows specifying regular-expression like patterns to match entities spanning multiple 'words'
* Allow choosing unicode codepoints for offsets instead of UTF-8 byte offsets ([#15](https://github.com/proycon/analiticcl/issues/15))

Bugfixes:
* use lowest frequency of either variant or target when using variant lists (https://github.com/knaw-huc/golden-agents-htr#15)
* Allow out-of-vocabulary words in LM; not everything that's in the lexicons has to necessarily also in the LM





# v0.3.3 - 2022-02-15

Important bugfix release:

* Fixed use of frequency information in score() function
* Fixed parsing of DistanceThreshold (edit distance threshold, anagram distance threshold), when it consist of a relative and absolute component.
* Better parameter validation in Python binding
* More verbose feedback on chosen parameters
* Fixed version information






# v0.3.2 - 2022-02-02

* fixed auto-detection of frequency information in parsing variant lists
* fix for the python wheel building





# v0.3.1 - 2021-10-06

Minor bugfix release: fixes an issue with invalid JSON serialisation [#13](https://github.com/proycon/analiticcl/issues/13) 





# v0.3.0 - 2021-09-24

Major development updates:

* Initial implementation on finding matches in running text (error detection); search mode [#2](https://github.com/proycon/analiticcl/issues/2)
   * Support for Language Models to consider context
   * Support for n-grams; decoding using Finite State Transducers
   * Strict separation between lexicon and language model
   * Still experimental
* Removed frequency from score component and added it as a separate score
   * Added frequency-ranking as an opt-in feature now; explicitly propagate frequency score and distance score separately to the output
* Removed lexicon weights
* Made distance score computations relative to input length
* Changed default weights so levenshtein-damarau carries most weight
* Implemented a Python binding ([#1](https://github.com/proycon/analiticcl/issues/1))
* Fixed insertions after deletion ([#6](https://github.com/proycon/analiticcl/issues/6)), removed premature bound-check optimisations
* Implemented a learning mode that collects variants for a given lexicon, either in running text or matched against another test lexicon
* Implemented a cut-off threshold
* Allow frequency information in variant lists
* Adhere strict to lexiconc/variantlist loading order as specified on command line
* Return all matching lexicons for matching rather than just one (in case an entry exists in multiple lexicons)
* More debug levels
* Anagram/edit distance can now be set to an absolute value or a ratio (relative to input length)
* Significant documentation updates






# v0.2.0 - 2021-05-04

* This release replaces the underlying big integer library with ibig 0.3.2, which leads to a significant performance increase due to less heap allocations.
* Implemented explicit variant ingestion and matching  (but still requires proper testing)
* fixed benchmarks
* allow some escape sequences in alphabet files






# v0.1.1 - 2021-04-30

Bugfix release





# v0.1.0 - 2021-04-30

Initial experimental release of analiticcl
