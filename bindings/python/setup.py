from setuptools import setup
from setuptools_rust import Binding, RustExtension

extras = {}
extras["testing"] = ["pytest"]

setup(
    name="analiticcl",
    version="0.3.0",
    description="Analiticcl is an approximate string matching or fuzzy-matching system that can be used to find variants for spelling correction or text normalisation",
    long_description=open("README.md", "r", encoding="utf-8").read(),
    long_description_content_type="text/markdown",
    keywords="NLP, spelling correction, normalisation",
    author="Maarten van Gompel",
    author_email="proycon@anaproy.nl",
    url="https://github.com/proycon/analiticcl",
    license="GPLv3",
    rust_extensions=[RustExtension("analiticcl.analiticcl", binding=Binding.PyO3, debug=False)],
    extras_require=extras,
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "Intended Audience :: Science/Research",
        "License :: OSI Approved :: GNU General Public License v3 (GPLv3)",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.5",
        "Programming Language :: Python :: 3.6",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Topic :: Text Processing :: Linguistic",
    ],
    packages=[
        "analiticcl",
    ],
    zip_safe=False,
)
