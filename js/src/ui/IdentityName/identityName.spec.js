// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import React from 'react';
import { mount } from 'enzyme';
import sinon from 'sinon';

import IdentityName from './identityName';

const ADDR_A = '0x123456789abcdef0123456789A';
const ADDR_B = '0x123456789abcdef0123456789B';
const ADDR_C = '0x123456789abcdef0123456789C';
const STORE = {
  dispatch: sinon.stub(),
  subscribe: sinon.stub(),
  getState: () => {
    return {
      balances: {
        tokens: {}
      },
      personal: {
        accountsInfo: {
          [ADDR_A]: { name: 'testing' },
          [ADDR_B]: {}
        }
      }
    };
  }
};

function render (props) {
  return mount(
    <IdentityName
      store={ STORE }
      { ...props } />
  );
}

describe('ui/IdentityName', () => {
  describe('rendering', () => {
    it('renders defaults', () => {
      expect(render()).to.be.ok;
    });

    describe('account not found', () => {
      it('renders null with empty', () => {
        expect(render({ address: ADDR_C, empty: true }).html()).to.be.null;
      });

      it('renders address without empty', () => {
        expect(render({ address: ADDR_C }).text()).to.equal(ADDR_C);
      });

      it('renders short address with shorten', () => {
        expect(render({ address: ADDR_C, shorten: true }).text()).to.equal('123456…56789c');
      });

      it('renders unknown with flag', () => {
        expect(render({ address: ADDR_C, unknown: true }).text()).to.equal('UNNAMED');
      });
    });
  });
});
