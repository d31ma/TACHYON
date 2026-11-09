use strict;
use warnings;
use JSON::PP;

my $json = JSON::PP->new->canonical;
my $input = do { local $/; <STDIN> };
my $request = $json->decode($input || '{}');
my $body = $json->encode({ language => 'Perl', route => $request->{route} });
print $json->encode({ status => 200, headers => { 'content-type' => ['application/json'] }, body => $body });
